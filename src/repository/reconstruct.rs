//
// src/toolbox/repository
//
// Retrieve the contents of a managed file from the repository
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use anyhow::{Result, bail};
use crate::error;


/// Retrieve the contents of a managed toolbox file 
///
/// # Arguments
///
/// * `path` - path to the managed directory, relative to the repository root
/// * `spec` - revision spec (empty means index)
///
/// # Notes
///
/// The files are retrieved in the natural order of their paths. 
pub(super) fn reconstruct<P, S>(repo: &git2::Repository, path: P, rev: S) -> Result<Vec<u8>>  
where 
    P : AsRef<str>,
    S : AsRef<str>
{
    if rev.as_ref().is_empty() {
        // we are searching the index
        reconstruct_from_index(repo, path)
    } else {
        // we are searching a revision
        reconstruct_from_rev(repo, path, rev)
    }
}

/// Retrieve the contents of a managed toolbox file from index
///
/// # Notes
///
/// Retrieving files from git index is tricky since the directory structure ( a git
/// tree) is only written when a commit is created. This means that we cannot easily
/// access the contents of a partially staged directory. 
///
/// As a work-around, I am using `Pathspec` to get all the file paths that reside 
/// in the index (either staged or transitively), sort them, map them to the ids and 
/// finally map them to blobs. This requires multiple traversals of git database, 
/// so its rather inneficient when we are dealin with thousands of files. 
///
/// Maybe there is a better way of doing it by inspecting the index manually and 
/// matchign the index entries... but I am not doing it. 
fn reconstruct_from_index<P>(repo: &git2::Repository, path: P) -> Result<Vec<u8>>  
where 
    P : AsRef<str>
{
    let path = path.as_ref();

    // accumulator for all the blob contents (with dictionary header)
    let mut content = b"\\_sh v3.0  864  Dictionary\n".to_vec();
        
    let index = repo.index().map_err(error::OtherGitError::from)?;
        
    // apply the pathspec to the index
    let pathspec = git2::Pathspec::new(std::iter::once(path))
        .map_err(error::OtherGitError::from)?;
    let matches = pathspec.match_index(&index, git2::PathspecFlags::DEFAULT)
        .map_err(error::OtherGitError::from)?;
    // collect and sort the matched paths
    let mut paths = Vec::<&str>::new();

    for entry in matches.entries() {
        // only collect txt files
        if !entry.ends_with(b".txt") { continue; }
        
        // the repository should not contain non-unicode paths
        let path = match std::str::from_utf8(entry) {
            Err( _ )=> {
                // invalid path in the repository
                // print an error and continue
                let err = error::InvalidClobPath {
                    path: String::from_utf8_lossy(entry).into_owned()
                };    
                stderr!("{}", err);
                continue
            },
            Ok(path) => {
                path
            } 
        };
        // add the entry to the path collections
        paths.push(path);
    }

    if paths.is_empty() {
        bail!( 
            error::GitObjNotFound {
                path : path.to_owned(),
                rev  : "the index".to_owned()
            }
        );
    }

    // sort the paths in natural order
    alphanumeric_sort::sort_str_slice(paths.as_mut_slice());
    // retrieve the blob 
    for path in paths.into_iter() {
        let entry = index.get_path(std::path::Path::new(path), 0).ok_or_else(|| {
            error::GitObjNotFound {
                path : path.to_owned(),
                rev  : "the index".to_owned()
            }
        })?;
        let blob = repo.find_blob(entry.id).map_err(error::OtherGitError::from)?;
        // push it to the list
        if !content.is_empty() {
            content.extend(b"\n");
        }
        content.extend(blob.content());
    }

    Ok( content )
}

/// Retrieve the contents of a managed toolbox file from a revision
///
/// # Notes
///
/// This is an straightforward efficient implementation where we directly
/// walk a tree in a commit, sorting entries as we go. 
pub fn reconstruct_from_rev<P, S>(repo: &git2::Repository, path: P, rev: S) -> Result<Vec<u8>>  
where 
    P : AsRef<str>,
    S : AsRef<str>
{
    let path = path.as_ref();
    let rev = rev.as_ref();

    // accumulator for all the blob contents (with dictionary header)
    let mut content = b"\\_sh v3.0  864  Dictionary\n".to_vec();
    
    // find the object at the path 
    let tree = repo.revparse_single(&format!("{}:{}", rev, path))
        .map_err(error::OtherGitError::from)?;

    // which should be a tree
    let tree = tree.into_tree()
        .map_err(|_| {
            error::OtherGitError {
                msg : format!("'{}:{}' is not a directory in the git repository", rev, path)
            }
        })?;

    collect_blobs_in_natural_order(tree, repo, &mut |blob : git2::Blob| {
        // push it to the list
        if !content.is_empty() {
            content.extend(b"\n");
        }
        content.extend(blob.content());
    })?;
    

    Ok( content )
}


/// Internal iterator that yields blobs in a git tree, sorted naturally by path
fn collect_blobs_in_natural_order<'a, F>(
    tree: git2::Tree, repo: &'a git2::Repository, callback: &mut F
) -> Result<(), git2::Error> 
where 
    F: FnMut(git2::Blob<'a>)
{
    // collect and sort the entris by their path 
    let mut entries = tree.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        alphanumeric_sort::compare_str(a.name().unwrap_or(""), b.name().unwrap_or(""))
    });

    // walk the entires
    for entry in entries.into_iter() {
        match &entry.kind() {
            // if this is a tree, we collect blobs from here recursively
            Some(git2::ObjectType::Tree) => {
                collect_blobs_in_natural_order(
                    entry.to_object(repo)?.into_tree().expect("Git object type mismatch error"),
                    repo, 
                    callback
                )?;
            },
            // if this is an txt blob, yield it
            Some(git2::ObjectType::Blob) if entry.name().unwrap_or_default().ends_with(".txt") => {
                callback(
                    entry.to_object(repo)?.into_blob().expect("Git object type mismatch error")
                );
            },
            _ => {
                // ignore the rest
            }
        }

    }

    Ok( () )
}