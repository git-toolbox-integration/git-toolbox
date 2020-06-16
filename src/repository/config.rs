//
// src/toolbox/repository
//
// Repository configuration. 
//
// Utiltities for ensurign that the git repository configuration matches 
// the toolbox-git configuration. 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use crate::config::{Config, CONFIG_FILE};
use anyhow::{Result, bail};
use crate::error;

// git configuration keys we need to have set
const GIT_CONFIG: [(&str, &str); 3] = [
    ("filter.toolbox-filter.clean", "git-toolbox gitfilter --clean %f"),
    ("filter.toolbox-filter.smudge", "git-toolbox gitfilter --smudge %f"),
    ("filter.toolbox-filter.required", "true")
];

// git filter attribute we need to set on managed files
const GIT_FILTER_ATTR: & str = r"filter=toolbox-filter";

// comment to put in the gitattributes file
const GIT_COMMENT: & str = concat!(
    "# this section is managed by git-toolbox. Please do not edit below this line!"
);


lazy_static::lazy_static! {
    static ref GIT_FILTER_ATTR_REGEX : regex::Regex = regex::Regex::new(
        &format!(r"\b{}\b", GIT_FILTER_ATTR)
    ).expect("fatal: invalid regex");
}


use git2::Repository;
use std::path::{Path, PathBuf};
use std::convert::TryFrom;
use crate::cli_app::style;


/// Get the validated configuration for this repository
///
/// This function checks if the repository configuration has changed
/// and returns an apropriate diagnostic message in this case
pub(super) fn get_validated_config(repo: &Repository) -> Result<Config> {
    use crate::util::c_escape_str;

    // attempt to read the local configuration file
    let workdir = repo.workdir().expect("fatal: unable to retrieve git working directory");
    let local_config = try_read_local_config(workdir)?;

    // atempt to read the indexed configuration file
    let staged_config = try_read_staged_config(repo)?;

    // check if configuration file has changed 
    let config = match (local_config, staged_config) {
        // local and staged  match
        ( Some(local), Some(staged) ) if local == staged => {
            local
        },
        // local exists and is different from the staged 
        ( Some(_), _ ) => {
            bail!(error::ConfigurationChanged);
        },      
        // local does not exist      
        ( None, _ ) => {
            bail!(error::ConfigurationMissing);
        }
    };
    
    // parse the configuration file
    let config = Config::try_from(config.as_slice())?;

    // validate the git repository configuration
    let git_config = repo.config().map_err(error::OtherGitError::from)?;

    // check that all the requested keys exist and have the correct value
    GIT_CONFIG.iter().try_for_each(|(key, value)| {
        // retrieve the entry 
        let config_entry = git_config.get_entry(key).ok()?;    
        // check that the value is correct
        config_entry.value().and_then(|val| {
            if val.trim() == value.trim() { Some( () ) } else { None }
        })
    }).ok_or_else(|| {
        error::ConfigurationNeeded
    })?;

    // validate the git attributes
    let attributes = read_git_attributes(repo)?;

    // collect all the patterns that have the managed filter set
    let mut patterns = attributes.lines().filter_map(|line| {
        let (pattern, attrs) = parse_git_attribute_line(line);

        if GIT_FILTER_ATTR_REGEX.is_match(attrs) {
            Some(pattern)
        } else {
            None
        }
    }).collect::<std::collections::HashSet<_>>();

    // for each managed toolbox file, check if there is a matching pattern (and remove it)
    config.dictionaries.iter().map(|cfg| cfg.path.as_str()).try_for_each(|path| {
        if patterns.remove(path) || patterns.remove(c_escape_str(path).as_str()) {
            Ok( () )   
        } else {
            Err( error::ConfigurationNeeded )
        }
    })?;

    // if there are patterns left, configuration is needed!
    if !patterns.is_empty() {
        bail!{
            error::ConfigurationNeeded
        }
    }

    // we seem to be fine!
    Ok( config )
} 


/// Configure the repository
///
/// This function makes sure that the repository configuration is up to date.
/// This includes the following:
///
/// - check that the configuration file exist and add it to the index area if
///   nessesary
///
/// - check that the git filter configuration is up to date and update if if
///   nessesary
///
/// - check that the git attributes configuration is up to date and update if if
///   nessesary
///
pub(super) fn configure_repository(repo: &mut Repository) -> Result<()> {
    use std::collections::HashSet;
    use crate::util::c_escape_str;
    use itertools::Itertools;

    // attempt to read the local configuration file
    let workdir = repo.workdir().expect("fatal: unable to retrieve git working directory");
    let local_config = try_read_local_config(workdir)?.ok_or_else(|| {
        error::ConfigurationMissing
    })?;

    // parse the configuration file
    let config = Config::try_from(local_config.as_slice())?;

    // check if the config file needs staging (index version is either different or 
    // does not exist)
    if try_read_staged_config(repo)?.map(|staged| staged != local_config).unwrap_or(true) {
        // add the config file to the index
        let mut index = repo.index().map_err(error::OtherGitError::from)?;
        index.add_path(Path::new(CONFIG_FILE)).map_err(error::OtherGitError::from)?;
        index.write().map_err(error::OtherGitError::from)?;

        // write the diagnostic message
        stdout!("{} {}", 
            style("✓").green(),
            style(format!("git add {}", CONFIG_FILE)).bold()
        );
    }

    // update the git config
    let mut git_config = repo.config().map_err(error::OtherGitError::from)?;

    for (key, value) in GIT_CONFIG.iter() {
        git_config.set_str(key, value).map_err(error::OtherGitError::from)?;
    };

    stdout!("{} updated git config file", style("✓").green());

    // update the git attributes

    // read the attributes
    let attributes = read_git_attributes(repo)?;

    // build a set of managed paths (we use them to match the lines in git attributes file)
    let managed_paths = config.dictionaries.iter().flat_map(|cfg| {
        use std::iter::once;
        // produce both a ccopy of the path and the escaped version of the path
        // since we don't know whcih one is used
        //
        // the once() dance is needed since we can't turn slice into an iterator
        once(cfg.path.clone()).chain(once(c_escape_str(&cfg.path)))
    }).collect::<HashSet<String>>();

    // process the attributes
    let attributes = attributes.lines()
        // remove all managed patterns
        .filter_map(|line| {
            // filter the line contents
            match parse_git_attribute_line(line) {
                // remove lines matching one of the managed patterns
                (pattern, _) if managed_paths.contains(pattern)   => None, 
                // remove lines matching the managed atribute
                (_, attr) if GIT_FILTER_ATTR_REGEX.is_match(attr) => None, 
                // remove managed comment
                _         if line.trim() == GIT_COMMENT           => None,
                // otherwise we want to keep this line
                _                                                 => Some(line.to_owned())
            }
        })
        // add the new patterns for the managed files
        .chain({
            // generate one line per managed dictionary
            let new_patterns = config.dictionaries.iter().map(|cfg| 
                format!("{} {}", c_escape_str(&cfg.path), GIT_FILTER_ATTR)
            );

            // emit the items
            std::iter::once(GIT_COMMENT.to_owned()).chain(new_patterns)
        })
        // add all lines together
        .join("\n");

    // write the new attributes
    write_git_attributes(&attributes, repo)?;

    stdout!("{} updated git attributes file", style("✓").green());

    Ok( () )
}


/// Locate and retrieve the contents of the local configuration file
fn try_read_local_config<P: AsRef<Path>>(workdir: P) -> Result<Option<Vec<u8>>> {
    use std::fs;
    
    // path to the local configuration file
    let path = workdir.as_ref().to_owned().join(CONFIG_FILE);

    // read the file and map the errors 
    fs::read(&path)
        .map(Some)
        // remap not found error to None
        .or_else(|err| {
            match err.kind() {
                std::io::ErrorKind::NotFound => Ok( None ),
                _                            => Err( err )
            }
        })
        // error message
        .map_err(|err| {
            error::FileReadError {
                path,
                msg  : err.to_string()
            }
            // map it to anyhow::Error
            .into()
        })
}

/// Locate and retrieve the contents of the staged configuration file
fn try_read_staged_config(repo: &Repository) -> Result<Option<Vec<u8>>>  {
    repo.index()
        .and_then(|index| {
            index
                // find the entry and extract the result
                .get_path(Path::new(CONFIG_FILE), 0)
                .map(|entry| repo.find_blob(entry.id))
                // transform Option<Result> to Result<Option>
                .transpose()
                .map(|maybe_blob| maybe_blob.map(|blob| blob.content().to_vec()))
        })
        // remap error messages
        .map_err(|err| error::OtherGitError::from(err).into())
}


fn git_attributes_path(repo : &Repository) -> PathBuf {
    repo.path().to_owned().join("info/attributes")
}

fn read_git_attributes(repo: &Repository) -> Result<String> {
    use std::fs;

    let path = git_attributes_path(repo);

    fs::read_to_string(&path)
        // remap not found error to empty string
        .or_else(|err| {
            match err.kind() {
                std::io::ErrorKind::NotFound => Ok( String::new() ),
                _                            => Err( err )
            }
        })
        // error message
        .map_err(|err| {
            error::FileReadError {
                path,
                msg  : err.to_string()
            }
            // map it to anyhow::Error
            .into()
        })
}

fn write_git_attributes(text: &str, repo: &mut Repository) -> Result<()> {
    use std::fs;

    let path = git_attributes_path(repo);

    fs::write(&path, text)
        // error message
        .map_err(|err| {
            error::FileWriteError {
                path,
                msg  : err.to_string()
            }
            // map it to anyhow::Error
            .into()
        })
}


// need support for git attribute files... 
fn parse_git_attribute_line(line: &str) -> (&str, &str) {
    let line = line.trim();

    let prefix_end = if line.starts_with('"') {
        // this is an escaped string
        let mut escaped = true;
        let mut end = None;

        for (index, ch) in line.char_indices() {
            match ch {
                '"' if !escaped => {
                    end = Some(index+1);
                    break;
                },
                '\\' => {
                    escaped = !escaped;
                }, 
                _ => {
                    escaped = false;
                }
            }
        }

        end.unwrap_or_else(|| line.len())
    } else {
        // this is an unescaped string
        line.find(' ').unwrap_or_else(|| line.len())
    };

    line.split_at(prefix_end)
}