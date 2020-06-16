//
// src/toolbox/repository
//
// Utilities for computing the list of changes in managed files relative
// to the git repository
//
// Also defines CLOBs - text blobs. CLOBs are the central concept in 
// git-toolbox since managed files are decomposed into series of CLOBS
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use super::Repository;

/// A text data object stored in a filesystem
#[derive(Debug)]
pub struct Clob {
    /// The path where these records should be stored
    pub path    : String,
    /// The clob contents
    pub content : String
}

type ClobStream = Box<dyn Iterator<Item = Clob>>;


/// A filesystem update action
#[derive(Debug)]
pub enum ClobDiff {
    Add { clob: Clob },
    Update { clob: Clob },
    Delete { path: String }
}

// Clob validation error
pub enum ClobValidationIssue {
    AddedInWorkdir   { path: String },
    DeletedInWorkdir { path: String },
    UpdatedInWorkdir { path: String },
    InvalidPath      { path: Vec<u8> } 
}

/// Diff summary
pub struct DiffStats {
    pub added   : usize, 
    pub changed : usize, 
    pub deleted : usize
}

use anyhow::Result;
use crate::error;

impl Repository {
    /// Checks the contents of a managed folder for external modifications
    ///
    /// This will run a git status check on a managed folder and pick any
    /// *.txt file that was changed in the working directory
    ///
    /// Note: this won't catch external changes if they have been added to 
    /// the index
    pub fn validate_clobs_in_workdir<P>(&self, root: P) -> Result<Vec<ClobValidationIssue>>
    where 
        P: AsRef<str>

    {
        use git2::StatusOptions;

        let repo  = &self.repository;

        // query the status of the files at the path
        let statuses = {
            let mut status_options = StatusOptions::new();
            status_options.pathspec(root.as_ref());
            status_options.include_ignored(false);

            repo.statuses(Some(&mut status_options)).map_err(error::OtherGitError::from)?
        };

        // iterate the status entries, picking the entries that show external modification
        let issues = statuses.iter().filter_map(|entry| {
            // ignore anythign that is not a txt file
            if !entry.path_bytes().ends_with(b".txt") { return None }

            // validate the path 
            // it should be ASCII only
            let path = match entry.path().filter(|p| p.is_ascii()) {
                Some( path ) => {
                    path
                },
                None => {
                    let issue = ClobValidationIssue::InvalidPath {
                        path : entry.path_bytes().to_owned()
                    };

                    return Some(issue)
                }
            };

            // map statuses to issues
            match entry.status() {
                st if st.is_wt_new() => {
                    Some(
                        ClobValidationIssue::AddedInWorkdir   { path : path.to_owned() }
                    )
                },
                st if st.is_wt_modified() || st.is_wt_typechange() => {
                    Some(
                        ClobValidationIssue::UpdatedInWorkdir { path : path.to_owned() }
                    )
                },
                st if st.is_wt_deleted() || st.is_wt_renamed() => {
                    Some(
                        ClobValidationIssue::DeletedInWorkdir { path : path.to_owned() }
                    )
                },
                // no unintended modifications (maybe)
                _ => {
                    None
                }
            }
        })
        .collect();

        Ok( issues )
    }


    /// Checks the contents of a managed folder for being staged
    ///
    /// This will run a git status check on a managed folder and pick any
    /// *.txt file that were changed in the index
    pub fn get_staged_clobs<P>(&self, root: P) -> Result<Vec<ClobDiff>>
    where 
        P: AsRef<str>
    {
        use git2::StatusOptions;

        let repo  = &self.repository;

        // query the status of the files at the path
        let statuses = {
            let mut status_options = StatusOptions::new();
            status_options.pathspec(root.as_ref());
            status_options.include_ignored(false);

            repo.statuses(Some(&mut status_options)).map_err(error::OtherGitError::from)?
        };

        // iterate the status entries, picking the entries that were changed in the index
        let diff = statuses.iter().filter_map(|entry| {
            // ignore anythign that is not a txt file
            if !entry.path_bytes().ends_with(b".txt") { return None }

            // validate the path 
            // it should be ASCII only
            // we silently ignore invalid entries
            let path = entry.path().filter(|p| p.is_ascii())?;

            // map statuses to issues
            match entry.status() {
                st if st.is_index_new() => {
                    Some(
                        ClobDiff::Add {
                            clob: Clob {
                                path    : path.to_owned(),
                                content : String::new() // don't care about the content
                            }
                        }
                    )
                },
                st if st.is_index_modified() || st.is_index_typechange() => {
                    Some(
                        ClobDiff::Update {
                            clob: Clob {
                                path    : path.to_owned(),
                                content : String::new() // don't care about the content
                            }
                        }

                    )
                },
                st if st.is_index_deleted() || st.is_index_renamed() => {
                    Some(
                        ClobDiff::Delete {
                            path    : path.to_owned()
                        }

                    )
                },
                // no unintended modifications (maybe)
                _ => {
                    None
                }
            }
        })
        .collect();

        Ok( diff )
    }
    /// Performs a diff of the clobs and the repository and returns a list
    /// of file actions required to update the clob state
    pub fn diff_clobs_at_path<P>(&self, root: P, clobs: ClobStream) -> Result<Vec<ClobDiff>> 
    where 
        P: AsRef<str>
    {
        use git2::{Oid,StatusOptions,ObjectType};

        let root = root.as_ref();

        let repo  = &self.repository;
        let index = self.repository.index().map_err(error::OtherGitError::from)?;

        // the set of clobs at the path
        //
        // we use this to detect which clobs are updated and which have been deleted
        let mut clobset = std::collections::HashSet::new(); 

        // query the status of the files at the path
        let statuses = {
            let mut status_options = StatusOptions::new();
            status_options.pathspec(root);
            status_options.include_unmodified(true); 
            status_options.include_ignored(false);

            repo.statuses(Some(&mut status_options)).map_err(error::OtherGitError::from)?
        };

        for status in statuses.iter() {
            // ignore anythign that is not a txt file
            if !status.path_bytes().ends_with(b".txt") { continue }
            // ignore files that are deleted or renamed in the index
            if status.status().is_index_deleted() { continue }
            
            // TODO: detect cases where the contents have been tampered with

            // get the path, reporting an error if it is not valid unicode
            let path = status.path().ok_or_else(|| {
                let path = String::from_utf8_lossy(status.path_bytes()).into_owned();

                error::InvalidManagedPath {
                    path
                }
            })?
            .to_lowercase();

            clobset.insert(path);
        };

        // the list of actions to perform
        let mut diff_list = vec!();
        
        // walk the clobs and update the changed ones
        for clob in clobs {
            // update the clob path by adding the root prefix
            let clob = Clob {
                path: format!("{}/{}", &root, &clob.path),
                ..clob
            };

            // mark this clob as resolved
            clobset.remove(&clob.path.to_lowercase());

            // and build the diff
            let clob_diff = match index.get_path(std::path::Path::new(&clob.path), 0) {
                // the entry exists, check if the content has changed
                Some(entry) => {
                    // compute the clob hash
                    let oid = Oid::hash_object(ObjectType::Blob, clob.content.as_bytes())?;
                    // the content has changed if the id OR the content itself has changed
                    let clob_contents = clob.content.as_bytes();
                    if oid != entry.id || repo.find_blob(entry.id)?.content() != clob_contents {
                        Some(ClobDiff::Update { clob })
                    } else {
                        None
                    }
                },
                // no such entry
                None => {
                    Some(ClobDiff::Add { clob })
                }
            };

            // add the diff to the diff list
            if let Some(diff) = clob_diff {
                diff_list.push(diff);
            }
        }

        // all files still in the set must have been deleted
        for path in clobset {
            // save the file change action
            diff_list.push( ClobDiff::Delete { path } );
        }

        Ok( diff_list )
    } 
}

impl Clob {
    pub fn validated(self) -> Self {
        assert!(self.path.is_ascii(), 
            "fatal - non-ascii CLOB name '{}' violates internal assumttions", 
            &self.path
        );

        self
    }
}


impl ClobDiff {
    pub fn diff_marker(&self) -> &str {
        match self {
            ClobDiff::Add { clob: _}      => "added   ",
            ClobDiff::Update { clob: _}   => "modified",
            ClobDiff::Delete { path : _ } => "deleted "
        }
    }

    pub fn display_diff_marker(&self) -> impl std::fmt::Display {
        use crate::cli_app::style;

        match self {
            ClobDiff::Add { clob: _}      => style("added   ").green(),
            ClobDiff::Update { clob: _}   => style("modified").yellow(),
            ClobDiff::Delete { path : _ } => style("deleted ").red()
        }
    }

    pub fn filename(&self) -> &str {
        let path = self.path();

        path.rsplit('/').next().expect("internal error: clob is not a file")
    }

    pub fn path(&self) -> &str {
        match self {
            ClobDiff::Add { clob } | ClobDiff::Update { clob }  => {
                &clob.path                
            },
            ClobDiff::Delete { path } => {
                &path
            }
        }
    }

}

impl ClobValidationIssue {
    pub fn path(&self) -> &str {
        match self {
            ClobValidationIssue::AddedInWorkdir   { path } |
            ClobValidationIssue::DeletedInWorkdir { path } |
            ClobValidationIssue::UpdatedInWorkdir { path } => {
                path
            }, 
            _ => {
                ""
            }
        }
    }
}


impl DiffStats {
    pub fn count(diff: &[ClobDiff]) -> Self {
        let mut added = 0;
        let mut changed = 0;
        let mut deleted = 0;

        for e in diff {
            match e {
                ClobDiff::Add { clob: _ } => { added+=1; },
                ClobDiff::Update { clob: _ } => { changed+=1; },
                ClobDiff::Delete { path: _ } => { deleted+=1; },
            }
        }

        DiffStats { added, changed, deleted } 
    }

    pub fn no_changes(&self) -> bool {
        self.added == 0 && self.changed == 0 && self.deleted == 0
    }
}


use std::fmt::{Display, Formatter};

impl Display for DiffStats {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        use crate::cli_app::style;

        if self.no_changes() {
            write!(formatter, "       {}", style("no changes").green())?;
        } else {
            write!(formatter, "{:>6} {} {:>6} {} {:>6} {}", 
                    self.added, style("added").green(),
                    self.changed, style("modified").yellow(),
                    self.deleted, style("deleted").red()
            )?;
        }
            

        Ok( () )
    }
}
