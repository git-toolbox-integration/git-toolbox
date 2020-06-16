//
// src/reset.rs 
//
// Implementation of git-toolbox reset 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use crate::repository::{Repository, ClobDiff, DiffStats};
use crate::toolbox::{Dictionary, ToolboxFileIssue};
use crate::config::DictionaryConfig;
use itertools::{Itertools, Either};
use crate::cli_app::style;

use crate::error;
use anyhow::{Result, bail};

const MAX_TO_SHOW: usize = 8;

struct ManagedFileSummary {
    // managed file name for displaying (relative to current folder)
    pub display_name  : String,
    // path to the file (relative to the repository)
    pub path          : String, 
    // path to the managed content
    pub contents_path : String,
    // the unstaged diff
    pub unstaged_diff : Vec<ClobDiff>,
    // the issues
    pub toolbox_issues : Vec<ToolboxFileIssue>,

}


pub fn reset(paths: Vec<String>, verbose: bool, force: bool) -> Result<()> {
    // load the repository
    let repo = Repository::open()?;

    // dictionary selection
    let dictionaries : Vec<&DictionaryConfig> = if paths.is_empty() {
        repo.config().dictionaries.iter().collect()
    } else {
        paths.iter().map(|path| {
            // convert the path to one relative to the repo
            let path = repo.get_path_relative_to_repo(path)?.to_string_lossy().into_owned();

            repo.config().dictionary_by_path(path)
        })
        .collect::<Result<Vec<_>>>()?
    };

    // process on the requested files
    let (summaries, errors) : (Vec<_>, Vec<_>) = dictionaries.into_iter().map(|cfg| {
        ManagedFileSummary::new(&repo, cfg)
    })
    // split off and collect sucesses and failures
    .partition_map(|result| -> Either<_, anyhow::Error> {
        match result {
            Ok( val )  => Either::Left(val),
            Err( err ) => Either::Right(err)
        }
    });

    // abort if there are errors
    if !errors.is_empty() {
        // collect all errors
        let err_msg = errors.into_iter().join("\n");

        bail!(
            "{}\n⚠️  There were errors. Aborting. No changes to the working directory were made", 
            err_msg
        );
    }

    // we are only interested in files that have changes
    let summaries: Vec<_> = summaries.into_iter().filter(|s| {
        s.any_unstaged() || s.missing_header()
    }).collect();

    // check if ther is any work to do
    if summaries.is_empty() {
        stdout!("✅ Nothing to do.");

        return Ok( () )
    }

    // print the unstaged changes
    for summary in summaries.iter() {
        summary.display_unstaged_diff(verbose);
    }

    if !force {
        let cmd = format!("git reset --force {}", paths.join(" "));

        bail!(concat!( 
                "⚠️  Resetting will discard any changes you have made to the files.\n",
                "      (if you understand this and still wish to proceed, use \"{}\")"
            ), style(cmd).bold()
        );
    }

    // reset all files
    for summary in summaries.iter() {
        let absolute_path = repo.workdir()?.to_owned().join(&summary.path);

        let data = Repository::reconstruct(&summary.contents_path, "")?;
        std::fs::write(&absolute_path, data).map_err(|err| {
            error::FileWriteError {
                path : absolute_path,
                msg  : err.to_string()
            }
        })?;


        let stats = summary.restore_stats();
        stdout!("{} Restored {} from git index ({} added, {} modified, {} deleted)",
            style("✓").green(),
            &summary.display_name,
            stats.added,
            stats.changed, 
            stats.deleted
        );
    }

    stdout!("\n✅  Reset {} managed toolbox dictionaries.", summaries.len());

    Ok( () )

}



impl ManagedFileSummary {
    pub fn new(repo :&Repository, cfg: &DictionaryConfig) -> Result<Self> {
        // the file path
        let path = cfg.path.clone();

        // load and split the dictionary
        let dictionary = Dictionary::load(&repo, cfg, false)?;

        // obtain the printable relative path to the file
        let display_name = crate::util::get_relative_path(
            repo.workdir()?.to_owned().join(&cfg.path)
        ).display().to_string();

        let contents_path = dictionary.contents_root();
        let (clobs, toolbox_issues) = dictionary.split();

        // run the diff 
        let unstaged_diff = repo.diff_clobs_at_path(&contents_path, clobs)?;


        // return the diff and the issues
        Ok( 
            ManagedFileSummary {
                display_name,
                path, 
                contents_path, 
                unstaged_diff,
                toolbox_issues
            }
        )

    }

    pub fn any_unstaged(&self) -> bool {
        !self.unstaged_diff.is_empty()
    }

    pub fn restore_stats(&self) -> DiffStats {
        let stats = DiffStats::count(&self.unstaged_diff);
       
        // invert the counts (we are restoring, not adding)
        DiffStats {
            added : stats.deleted,
            changed : stats.changed,
            deleted: stats.added
        }
    }

    pub fn missing_header(&self) -> bool {
        self.toolbox_issues.iter().any(|issue| {
            matches!(issue, ToolboxFileIssue::MissingDictionaryHeader { line: _ })
        })
    }


    pub fn display_unstaged_diff(&self, verbose: bool) {
        if !self.any_unstaged() { return }

        stdout!("\n  {}:\n", style(&self.display_name).italic());
        let to_show = if verbose { self.unstaged_diff.len() } else { MAX_TO_SHOW };
        for e in self.unstaged_diff.iter().take(to_show) {
            stdout!("        {} {}", e.display_diff_marker(), e.filename());
        }
        if to_show < self.unstaged_diff.len() {
            stdout!("        ...");
            stdout!("        ({} other changes, use \"{}\" to see all)", 
                self.unstaged_diff.len() - to_show,
                style("\"git status --verbose\"").bold()
            );
        }
        stdout!(""); 
    }
}


