//
// src/git_filter.rs 
//
// Implementation of git-toolbox gitfilter --clean 
//
// (C) 2020 Taras Zakharko
//
// This cod

use crate::repository::{Repository, MANAGED_FILE_TEXT};
use crate::toolbox::Dictionary;

use std::path::Path;
use std::io::Write;

use anyhow::{Result, bail};
use crate::error;


/// Git clean filter 
///
/// # Notes
///
/// The clean filter is run by git to transform the on-disk file to the blob
/// managed by git itself (for example when stagign the file or when checkign whether
/// the file has changed).
///
/// We highjack the clean filter to implement the following functionality:
///
///  - reject any attempts user makes to manually stage a managed file
///
///  - notify git that a managed file has changed on disk by supplying a useful 
///    diff message
///
/// This is accomplished in the following way: we first check if the repository
/// index lock is active. If it is, we assume that we are in the middle of an "add"
/// operation, so we abort with an error. If it is not acitve, we asume that the 
/// filter is run as part of `git status` or `git diff` etc. operation, so we return
/// a diff message instead. 
pub fn clean<P : AsRef<str>>(path: P) -> Result<()>  {
    // if the index is locked, we just return the error
    if Repository::check_for_lock()? {
        bail!(
            error::UnableToStageManagedFile {
                path : path.as_ref().to_owned().into()
            }   
        )
    };

    // run the actual clean filter which checks for the changes in the file
    // and generates a diff message
    //
    // if the inner filter fails, we don't want to abort the entire procedure
    // we just return a dummy message
    let mut report = do_clean(path).unwrap_or_default();

    // if the diff is empty, we want to output the standard content so that git thinks
    // the file did not change
    if report.is_empty() {
        report.push_str(MANAGED_FILE_TEXT);
    }

    // print it all to stdout
    let mut stdout = std::io::stdout();
    stdout.write_all(report.as_bytes()).expect("fatal - stdout error");

    Ok( () )
}

// The actual worker function
fn do_clean<P : AsRef<str>>(path: P) -> Result<String>  {
    // load the repository
    let repo = Repository::open()?;

    // transform it into the path relative to the repository
    let path = Path::new(path.as_ref());

    // it is safe to use lossy UTF-8 here since a managed file cannot have
    // non-utf-8 name anyway
    let repo_path = repo.get_path_relative_to_repo(path)?.to_string_lossy().into_owned();

    // retrieve the dictionary config
    let config = repo.config().dictionary_by_path(&repo_path)?;
    
    // load and split the dictionary 
    let (clobs, _) = Dictionary::load(&repo, config, false)?.split();
    // run the diff
    let mut changes = repo.diff_clobs_at_path(&format!("{}.contents", &config.path), clobs)?;
    changes.sort_by(|a, b| {
        alphanumeric_sort::compare_str(a.filename(), b.filename())
    });

    // build a report
    let report = changes.into_iter()
        .fold(String::new(), |mut diff, action| {
            diff.push_str(action.diff_marker());
            diff.push(' ');
            diff.push_str(action.filename());
            diff.push('\n');

            diff
        });

    Ok( report )
}