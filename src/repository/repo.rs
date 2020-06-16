use crate::config::Config;

/// The local repository
pub struct Repository {
    pub(super) repository : git2::Repository,
    pub(super) config     : Config
}   

use anyhow::Result;

use crate::error;
use std::path::{Path, PathBuf};

impl Repository {
    fn __open() -> Result<git2::Repository> {
        git2::Repository::open_from_env()
            // process errors
            .map_err(|err| {
                use git2::ErrorCode;

                match err.code() {
                    ErrorCode::NotFound => error::InvalidRepository.into(),
                    _                   => error::OtherGitError::from(err).into()
                }
            })
            // check that this is not a bare repository
            .and_then(|repository| -> Result<_>{
                if repository.is_bare() {
                    Err( error::InvalidRepository.into() )
                } else {
                    Ok( repository )
                }
            })
    }


    /// Open the repository connection
    pub fn open() -> Result<Repository> {
        // open the git repository
        let repository = Repository::__open()?;

        // retrieve the validated config
        let config = super::config::get_validated_config(&repository)?;

        // return the repository
        Ok(
            Repository {repository, config}
        )            
    }

    /// Confgure the repository
    pub fn configure() -> Result<()> {
        // open the git repository
        let mut repository = Repository::__open()?;

        // retrieve the validated config
        super::config::configure_repository(&mut repository)
    }

    /// Reconstruct a path
    /// 
    /// Path is assumed to be relative to the repository
    pub fn reconstruct<P, S>(path: P, rev: S) -> Result<Vec<u8>>  
    where 
        P : AsRef<str>,
        S : AsRef<str>
    {
        // open the git repository
        let repository = Repository::__open()?;

        // forward the reconstruct logic
        super::reconstruct::reconstruct(&repository, path, rev)
    }

    pub fn workdir(&self) -> Result<&Path> {
        self.repository.workdir().ok_or_else(|| {
            error::OtherGitError {
                msg: "unable to retrieve the working directory".to_owned()
            }.into()
        })
    }

    pub fn workdir_for_repo_here() -> Result<PathBuf> {
        let repo = Repository::__open()?;

        repo.workdir().map(|path| path.to_owned()).ok_or_else(|| {
            error::OtherGitError {
                msg: "unable to retrieve the working directory".to_owned()
            }.into()
        })
    }


    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn head_display_name(&self) -> String {
        use crate::cli_app::style;

        self.repository.head().map(|r| {
            String::from_utf8_lossy(r.shorthand_bytes()).into_owned()
        }).unwrap_or_else(|_| {
            style("<unknown>").red().to_string()
        })
    }

    /// Translate the path to one relative to the repo workign directory
    /// 
    /// # Notes
    ///
    /// It is an error if the path is outside the repo workign directory
    pub fn get_path_relative_to_repo<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        get_path_relative_to_root(path, self.workdir()?)
    }


    /// Translate the path to one relative to the repo workign directory
    /// 
    /// # Notes
    ///
    /// It is an error if the path is outside the repo workign directory
    pub fn get_path_relative_to_repo_here<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let repo = Repository::__open()?;
        let workdir = repo.workdir().ok_or_else(|| {
            error::OtherGitError {
                msg: "unable to retrieve the working directory".to_owned()
            }
        })?;

        get_path_relative_to_root(path, workdir)
    }

    /// Check if the git index is locked for writing without validating the configuration
    pub fn check_for_lock() -> Result<bool> {
        let repository = Repository::__open()?;
        let path  = repository.path().to_owned().join("index.lock");

        Ok( path.exists() )
    }
}



pub fn get_path_relative_to_root<P, R>(path: P, root: R) -> Result<PathBuf> 
where
    P: AsRef<Path>,
    R: AsRef<Path>
{
        use crate::util::absolute_path;

        let path = path.as_ref();

        // get the absolute path
        let absolute_path = absolute_path(path);

        // get the path relative to the repository
        let repo_path = absolute_path.strip_prefix(root.as_ref()).map_err(|_| {  
            error::PathNotInRepository {
                path : path.to_owned()
            } 
        })?;

        Ok( repo_path.to_path_buf() )
    }