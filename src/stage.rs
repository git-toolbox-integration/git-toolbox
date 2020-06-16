//
// src/stage.rs 
//
// Implementation of git-toolbox stage
// 
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use crate::repository::{Repository, ClobDiff, ClobValidationIssue};
use crate::toolbox::{Dictionary, ToolboxFileIssue};
use crate::config::DictionaryConfig;
use itertools::{Itertools, Either};
use crate::cli_app::style;

use crate::error;
use anyhow::{Result, bail};

const MAX_TO_SHOW: usize = 8;

struct StagedFileSummary {
    // managed file name for displaying (relative to current folder)
    pub display_name  : String,
    // path to the file (relative to the repository)
    pub path          : String, 
    // path to the managed content
    pub contents_path : String,
    // the unstaged diff
    pub unstaged_diff : Vec<ClobDiff>,
    // externally modified files
    pub workdir_issues : Vec<ClobValidationIssue>,
    // toolbox contents issues
    pub toolbox_issues : Vec<ToolboxFileIssue>
}


pub fn stage(paths: Vec<String>, verbose: bool, discard_workdir_changes: bool) -> Result<()> {
    // load the repository
    let mut repo = Repository::open()?;

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
        StagedFileSummary::new(&repo, cfg)
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
            "{}\n⚠️  There were errors. Aborting. No changes to the repository were made", 
            err_msg
        );
    }

    // check for external modifications in the working directory
    let any_workdir_issues = summaries.iter().any(StagedFileSummary::any_workdir_issues);

    if any_workdir_issues {
        stdout!("Some files managed by git-toolbox were externally modified.");

        stdout!(""); 
        
        for summary in summaries.iter() {
            summary.display_workdir_issues(verbose);
        }
    }

    // return an error if external files would be modified
    if !discard_workdir_changes && 
        summaries.iter().any(StagedFileSummary::workdir_changes_will_be_lost) 
    {
        // display an error message
        let err_msg = summaries.iter()
            .filter(|summary| summary.workdir_changes_will_be_lost())
            .map(|summary| {
                error::ExternalModificationsWillBeLost {
                    path: summary.contents_path.clone().into()
                }        
            })
            .join("\n");

        bail!(
            "{}\n\nUse {cmd} to force discarding any external modifications to managed files.", 
            err_msg, 
            cmd = style("\"git toolbox stage --discard-external-changes ...\"")
        );
    }
        
    // check if there is anythign to do
    if !summaries.iter().any(StagedFileSummary::any_unstaged) {
        stdout!("✅ No changes detected.");
        return Ok( () )
    }

    for summary in summaries.iter() {
        summary.display_unstaged_diff(verbose);
    }

    // apply the changes
    if let Err(err) = stage_changes(&mut repo, &summaries) {
        bail!(concat!(
                "\n{}\n\n",
                "⚠️  There were critical issues, aborting. Nothing added to be commited,",
                "contents of the managed folders might have changed."
            ),
            err
        )        
    };

    // print the toolbox issues
    let issue_count = summaries.iter().fold(0, |sum, summary| {
        sum + summary.toolbox_issues.len()
    });

    for summary in summaries.iter() {
        summary.display_toolbox_issues(verbose);
    }

    // print the final summary
    stdout!("");

    stdout!("\n✅ Added {} managed toolbox dictionaries to be commited.", 
        summaries.iter().filter(|s| s.any_unstaged()).count()
    );

    stdout!("");

    if issue_count != 0 {
        stdout!(concat!(            
                "⚠️  There were {} issues in toolbox dictionaries!",
                " Please check the list above and/or run {}."
            ),
            issue_count, 
            style("git status --verbose").bold()
        );
    }

    if any_workdir_issues {
        stdout!("⚠️  Some managed files were externally modified.");
    }


    Ok( () )

}

// helper to stage the repository
fn stage_changes(repo: &mut Repository, summaries: &[StagedFileSummary]) -> Result<()> {
    use indicatif::{ProgressBar, ProgressDrawTarget};
    use console::Term;

    let mut staging_area = repo.get_staging_area()?;

    // number of changes to apply
    let diff_count = summaries.iter().fold(0, |sum, summary| sum + summary.unstaged_diff.len());

    // prepare the progress bar
    let pb = ProgressBar::new(diff_count as u64);
    
    // we want to draw to stdout with max 10 updates per secocond
    let term = Term::stdout();

    pb.set_draw_target(ProgressDrawTarget::to_term(term.clone(), Some(10)));
    
    pb.set_style(indicatif::ProgressStyle::default_spinner()
        .template("  {spinner:.cyan/blue} {pos:>7}/{len} changes applied")
    );

    stdout!("Applying changes to the git repository index ...");

    // stage the affected toolbox files
    let (mut added, mut modified, mut deleted) = (0, 0, 0);
    for summary in summaries.iter().filter(|summary| summary.any_unstaged()) {
        staging_area.stage_managed_file(&summary.path)?;
        staging_area.stage_diffs(summary.unstaged_diff.iter(), |entry| {
            match entry {
                ClobDiff::Add { clob : _}    => added += 1,
                ClobDiff::Update { clob : _} => modified += 1,
                ClobDiff::Delete { path : _} => deleted += 1
            }

            pb.inc(1)
        })?;
    }

    // clean up the interactive part
    pb.finish_and_clear();
    if term.features().is_attended() {
        term.clear_last_lines(1).unwrap();
    }


    // collect the stats
    stdout!("{} Git index successfully updated ({} added, {} modified, {} deleted)",
        style("✓").green(),
        added,
        modified, 
        deleted
    );

    // commit the changes
    staging_area.commit()
}


impl StagedFileSummary {
    pub fn new(repo :&Repository, cfg: &DictionaryConfig) -> Result<Self> {
        // the file path
        let path = cfg.path.clone();

        // load and split the dictionary
        let dictionary = Dictionary::load(&repo, cfg, true)?;

        // obtain the printable relative path to the file
        let display_name = crate::util::get_relative_path(
            repo.workdir()?.to_owned().join(&cfg.path)
        ).display().to_string();

        let contents_path = dictionary.contents_root();
        let (clobs, toolbox_issues) = dictionary.split();

        // run the validation
        let workdir_issues = repo.validate_clobs_in_workdir(&contents_path)?;

        // run the diff 
        let unstaged_diff = repo.diff_clobs_at_path(&contents_path, clobs)?;


        // return the diff and the issues
        Ok( 
            StagedFileSummary {
                display_name,
                path, 
                contents_path,
                unstaged_diff,
                workdir_issues,
                toolbox_issues
            }
        )

    }

    pub fn any_workdir_issues(&self) -> bool {
        !self.workdir_issues.is_empty()
    }

    pub fn workdir_changes_will_be_lost(&self) -> bool {
        use std::collections::HashSet;

        let externally_modified_clobs = self.workdir_issues.iter()
            .map(ClobValidationIssue::path)
            .collect::<HashSet<_>>();

        // check if any of the changed clobss would overwrite
        // the external change
        self.unstaged_diff.iter().any(|clob| externally_modified_clobs.contains(clob.path()))
    }

    pub fn any_toolbox_issues(&self) -> bool {
        !self.toolbox_issues.is_empty()
    }

    pub fn any_unstaged(&self) -> bool {
        !self.unstaged_diff.is_empty()
    }

    pub fn display_toolbox_issues(&self, verbose: bool) {
        if !self.any_toolbox_issues() { return }

        stdout!("\n  Issues in {}:\n", style(&self.display_name).italic());
        let to_show = if verbose { self.toolbox_issues.len() } else { MAX_TO_SHOW };
        for e in self.toolbox_issues.iter().take(to_show) {
            stdout!("        {}", e);
        }
        if to_show < self.toolbox_issues.len() {
            stdout!("        ...");
            stdout!("        ({} other issues, use \"{}\" to see all)", 
                self.toolbox_issues.len() - to_show,
                style("git status --verbose").bold()
            );
        }
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


    pub fn display_workdir_issues(&self, verbose: bool) {
        use ClobValidationIssue::*;
        use std::collections::HashSet;

        if !self.any_workdir_issues() { return }

        let modified_clobs = self.unstaged_diff.iter()
            .map(ClobDiff::path)
            .collect::<HashSet<_>>();

        let to_show = if verbose { self.workdir_issues.len() } else { MAX_TO_SHOW };

        for e in self.workdir_issues.iter().take(to_show) {
            // check if this change would be discarded
            let discard_message = if modified_clobs.contains(e.path()) {
                "(change will be discarded)"
            } else {
                ""
            };

            match e {
                AddedInWorkdir { path } => {
                    stdout!("        {path}: {status} {msg}",
                        path = path, 
                        status = "new in the working directory",
                        msg = style(discard_message).red(),
                    );
                },
                UpdatedInWorkdir { path } => {
                    stdout!("        {path}: {status} {msg}",
                        path = path, 
                        status = "modified in working directory",
                        msg = style(discard_message).red(),
                    );
                },
                DeletedInWorkdir { path } => {
                    stdout!("        {path}: {status} {msg}",
                        path = path, 
                        status = "deleted in working directory",
                        msg = style(discard_message).red(),
                    );
                },
                InvalidPath { path } => {
                    use crate::util::escape_unicode_only;
;
                    stdout!("        {path}: {status}",
                        path = escape_unicode_only(&String::from_utf8_lossy(path)), 
                        status = style("invalid managed file path").red()
                    );
                }
            }
        }

        if to_show < self.workdir_issues.len() {
            stdout!("        ...");
            stdout!("        ({} other external changes, use \"{}\" to see all)", 
                self.workdir_issues.len() - to_show,
                style("\"git stage --verbose ...\"").bold()
            );
        }

        stdout!("");
    }
}


