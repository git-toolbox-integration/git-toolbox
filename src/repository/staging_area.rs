//
// src/toolbox/repository
//
// Git index mutation (aka staging area)
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use super::{Repository, MANAGED_FILE_TEXT, ClobDiff};
use std::marker::PhantomData;

use anyhow::Result;
use crate::error;

/// A repository updater
pub struct StagingArea<'repo> {
    repo    : PhantomData<&'repo mut Repository>,
    index   : git2::Index,
    workdir : &'repo std::path::Path
}


impl Repository {
     /// Get a staging area object for updating the repository
    pub fn get_staging_area(&mut self) -> Result<StagingArea> {
        let index = self.repository.index().map_err(error::OtherGitError::from)?;
        let workdir = self.workdir()?;         

        Ok(
            StagingArea {
                repo    : PhantomData,
                index,
                workdir
            }
        )
    }  
}

/// Represents the git staging area for the repository
///
/// The changes are only applied if they are commited
impl<'repo> StagingArea<'repo> {
    /// Apply the diffs to the staging area
    pub fn stage_diffs<'a, I, N>(&mut self, diffs: I, mut notify: N) -> Result<()> 
    where
        I : Iterator<Item = &'a ClobDiff>,
        N : FnMut(&ClobDiff)
    {
        use std::fs;
        use std::path::Path;
        use std::collections::HashSet;

        let workdir = self.workdir;

        // obtain the index
        let index = &mut self.index;

        // record paths at which deletion has occured, so that we can remove empty
        // folders afterwards
        let mut deleted_path_parents = HashSet::new();

        // run though the actions
        for diff in diffs {
            // run the callback
            notify(&diff);

            match diff {
                ClobDiff::Add { clob } | ClobDiff::Update {clob } => {
                    // construct the full path
                    let full_path = workdir.to_owned().join(&clob.path);

                    // write the file to the filesystem
                    std::fs::create_dir_all(
                        &full_path.parent().expect("fatal — missing prefix directory")
                    ).map_err(|err| {
                        error::FileWriteError {
                            path : full_path.clone(),
                            msg  : err.to_string()
                        }
                    })?;

                    fs::write(&clob.path, &clob.content).map_err(|err| {
                        error::FileWriteError {
                            path : full_path.clone(),
                            msg  : err.to_string()
                        }
                    })?;

                    // stage the file in the repository
                    index.add_path(Path::new(&clob.path)).map_err(error::OtherGitError::from)?;
                },
                ClobDiff::Delete { path } => {
                    let full_path = workdir.to_owned().join(&path);

                    // remove the file from the filesystem
                    fs::remove_file(&full_path).map_err(|err| {
                        error::FileDeleteError {
                            path : full_path.clone(),
                            msg  : err.to_string()
                        }
                    })?;

                    // remove the file from the repository
                    index.remove_path(Path::new(&path)).map_err(error::OtherGitError::from)?;

                    // mark this path 
                    if let Some(parent) = Path::new(&path).parent() {
                        deleted_path_parents.insert(parent.to_path_buf());        
                    }
                }
            }   
        }

        // delete the empty folders
        while !deleted_path_parents.is_empty() {
            // next iteration
            let mut parents = HashSet::new();

            for path in deleted_path_parents.into_iter() {
                // don't delete this path if it is the root
                if path.parent().is_none() { continue; }

                // get the full path
                let full_path = workdir.to_owned().join(&path);
                
                // try to remove it and, if successfull, add it to the next iteration
                if fs::remove_dir(&full_path).is_ok() {
                    if let Some(parent) = &path.parent() {
                        parents.insert(parent.to_path_buf());
                    }
                }
            }

            deleted_path_parents = parents;
        }

        Ok( () )
    }

    /// Add a managed file to the index
    ///
    /// # Notes
    ///
    /// - The real content of managed files is stored in the `.contents` directory
    /// and is reconstructed on the fly using the git filter. We put a placeholder
    /// text in the repository itself to alert the user if somethign went wrong. 
    /// 
    /// - Git checks whether a file has changed in the working directory by comparing
    /// it's stats with the ones in the index. This is a problem, since the placeholder
    /// text size is guaranteed to be different from the size of the actual file. To
    /// circumvent this, we have to change the file size of the index entry to match
    /// the actual file on disk. This makes `git status` and friends work correctly. 
    /// Since git does not seem to use the file size info in any other way, this should
    /// be safe
    ///
    /// - The API lacks any convenient way of constructing git index entries and doing
    /// it from scratch seems error-prone. We first stage the real file to have git
    /// build an entry for us and then replace it's contents by the placeholder
    /// API lacks any convenient way of doing it. This may create an orphaned blob
    /// in the database, but that is the price we have to pay
    pub fn stage_managed_file<P: AsRef<str>>(&mut self, path: P) -> Result<()> {
        use std::path::Path;

        let path = path.as_ref();

        // stage the real file to build the index entry
        self.index.add_path(Path::new(path)).map_err(error::OtherGitError::from)?;
        let entry = self.index.get_path(Path::new(path), 0).ok_or_else(|| {
            error::OtherGitError {
                msg : "unable to retrieve entry from index".to_owned()
            }
        })?;

        // save the file size
        let file_size = entry.file_size;

        // now re-add the same entry as a placeholder 
        self.index.add_frombuffer(&entry, MANAGED_FILE_TEXT.as_bytes())
            .map_err(error::OtherGitError::from)?;

        // add_frombuffer changes the file size, but we want to keep the size of the 
        // file on disk. So we need to do this dance one more time
        let mut entry = self.index.get_path(Path::new(path), 0).ok_or_else(|| {
            error::OtherGitError {
                msg : "unable to retrieve entry from index".to_owned()
            }
        })?;
        entry.file_size = file_size;
        self.index.add(&entry)?;

        Ok( () )
    }

    /// Write the git index, confirming any changes made to the staging area
    pub fn commit(mut self) -> Result<()> {
        self.index.write().map_err(error::OtherGitError::from)?;

        Ok( () )
    }

}

