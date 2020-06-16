//
// src/status.rs 
//
// Implementation of git-toolbox status 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use crate::repository::{Repository, ClobDiff, ClobValidationIssue, DiffStats};
use crate::toolbox::{Dictionary, ToolboxFileIssue};
use crate::config::DictionaryConfig;
use crate::cli_app::style;
use itertools::{Itertools, Either};

use anyhow::{Result,bail};


const MAX_TO_SHOW: usize = 8;

struct ManagedFileSummary {
    // managed file name for displaying (relative to current folder)
    pub display_name  : String,
    // path to the managed content
    pub contents_path : String,
    // the unstaged diff
    pub unstaged_diff : Vec<ClobDiff>,
    // the staged diff
    pub staged_diff : Vec<ClobDiff>,
    // externally modified files
    pub workdir_issues : Vec<ClobValidationIssue>,
    // toolbox contents issues
    pub toolbox_issues : Vec<ToolboxFileIssue>
}

pub fn status(files: Vec<String>, verbose: bool) -> Result<()> {
    assert!(files.is_empty());

    // open the repository
    let repo = Repository::open()?;

    // process on the requested files
    let (summaries, errors) : (Vec<_>, Vec<_>) = repo.config().dictionaries.iter().map(|cfg| {
        ManagedFileSummary::new(&repo, cfg)
    })
    // split off and collect sucesses and failures
    .partition_map(|result| -> Either<_, anyhow::Error> {
        match result {
            Ok( val )  => Either::Left(val),
            Err( err ) => Either::Right(err)
        }
    });
    
    if !errors.is_empty() {
        // collect all errors
        let err_msg = errors.into_iter().join("\n");

        bail!("{}\n⚠️  There were errors. Aborting.", err_msg);
    }

    stdout!("On branch {}", repo.head_display_name());

    // display work directory issues
    let any_workdir_issues = summaries.iter().any(ManagedFileSummary::any_workdir_issues);

    if any_workdir_issues {
        stdout!("\n{warning}: some files managed by git-toolbox were externally modified.",
            warning=style("warning").bold().yellow()
        );
        stdout!("  (these changes will be lost if you run {cmd})", 
            cmd = style("\"git toolbox stage\"").bold()
        );
        stdout!("  (if these changes are intended stage them manually using {cmd})",
            cmd = style("\"git add ...\"").bold()
        );

        stdout!("");

        for summary in summaries.iter() {
            summary.display_workdir_issues(verbose);
        }
    }

    // find the width of the file name for formatting 
    let max_display_path_width = summaries.iter().fold(0, |w, summary| {
        std::cmp::max(console::measure_text_width(&summary.display_name), w)
    });


    // staged diffs
    let any_staged = summaries.iter().any(ManagedFileSummary::any_staged);

    if any_staged {
        stdout!("Changes to be commited:");
        stdout!("");

        // display summaries
        for summary in summaries.iter() {
            stdout!("        {:<width$} : {}", 
                style(&summary.display_name).green(), 
                summary.staged_diff_stats(), 
                width=max_display_path_width
            );
        }

        // display diffs
        for summary in summaries.iter() {
            summary.display_staged_diff(verbose);
        }

        stdout!("");
    }

    // Unstaged changes
    stdout!("Changes not staged for commit:");
    stdout!(
        "  (use \"{}\" to stage the Toolbox dictionaries to be commited", 
        style("\"git toolbox stage\"").bold()
    );
    // stdout!(
    //     "  (use \"{}\" to discard local changes in the Toolbox dictionaries", 
    //     style("git toolbox reset").bold()
    // );
    stdout!("");


    // display summaries
    for summary in summaries.iter() {
        stdout!("        {:<width$} : {}", 
            &summary.display_name, 
            summary.unstaged_diff_stats(), 
            width=max_display_path_width
        );
    }

    // display diffs
    for summary in summaries.iter() {
        summary.display_unstaged_diff(verbose);
    }

    stdout!("");


    // display toolbox issues
    let issue_count = summaries.iter().fold(0, |sum, summary| {
        sum + summary.toolbox_issues.len()
    });

    for summary in summaries.iter() {
        summary.display_toolbox_issues(verbose);
    }
 
    stdout!("");

    if issue_count != 0 {
        stdout!("⚠️  There were {} issues in toolbox dictionaries! Please check the list above.", 
            issue_count
        );
    }
    if any_workdir_issues {
        stdout!("⚠️  Some managed files were externally modified. Please check the list above.");        
    }


    Ok( () )
}

impl ManagedFileSummary {
    pub fn new(repo :&Repository, cfg: &DictionaryConfig) -> Result<Self> {
        // load and split the dictionary
        let dictionary = Dictionary::load(&repo, cfg, false)?;

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

        // get the files already in index
        let staged_diff = repo.get_staged_clobs(&contents_path)?;

        // return the diff and the issues
        Ok( 
            ManagedFileSummary {
                display_name,
                contents_path,
                unstaged_diff,
                staged_diff,
                workdir_issues,
                toolbox_issues
            }
        )

    }

    pub fn any_workdir_issues(&self) -> bool {
        !self.workdir_issues.is_empty()
    }

    pub fn any_toolbox_issues(&self) -> bool {
        !self.toolbox_issues.is_empty()
    }

    pub fn any_staged(&self) -> bool {
        !self.staged_diff.is_empty()
    }

    pub fn any_unstaged(&self) -> bool {
        !self.unstaged_diff.is_empty()
    }

    pub fn unstaged_diff_stats(&self) -> DiffStats {
        DiffStats::count(&self.unstaged_diff)
    }

    pub fn staged_diff_stats(&self) -> DiffStats {
        DiffStats::count(&self.staged_diff)
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
    }

    pub fn display_staged_diff(&self, verbose: bool) {
        if !self.any_staged() { return }

        stdout!("\n  {}:\n", style(&self.display_name).italic().green());
        let to_show = if verbose { self.staged_diff.len() } else { MAX_TO_SHOW };
        for e in self.staged_diff.iter().take(to_show) {
            stdout!("        {} {}", 
                style(e.diff_marker()).green(), 
                style(e.filename()).green()
            )
        }
        if to_show < self.staged_diff.len() {
            stdout!("        ...");
            stdout!("        ({} other changes, use \"{}\" to see all)", 
                self.staged_diff.len() - to_show,
                style("\"git status --verbose\"").bold()
            );
        }
    }


    pub fn display_workdir_issues(&self, verbose: bool) {
        use ClobValidationIssue::*;

        if !self.any_workdir_issues() { return }

        let to_show = if verbose { self.workdir_issues.len() } else { MAX_TO_SHOW };

        for e in self.workdir_issues.iter().take(to_show) {
            match e {
                AddedInWorkdir { path } => {
                    stdout!("        {path}: {status}",
                        path = path, 
                        status = style("new in the working directory").red()
                    );
                },
                UpdatedInWorkdir { path } => {
                    stdout!("        {path}: {status}",
                        path = path, 
                        status = style("modified in working directory").red()
                    );
                },
                DeletedInWorkdir { path } => {
                    stdout!("        {path}: {status}",
                        path = path, 
                        status = style("deleted in working directory").red()
                    );
                },
                InvalidPath { path } => {
                    use crate::util::escape_unicode_only;

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
                style("\"git status --verbose\"").bold()
            );
        }

        stdout!("");
    }
}


