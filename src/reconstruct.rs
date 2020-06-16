//
// src/reconstruct.rs 
//
// Implementation of git-toolbox show 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use std::io::Write;

use crate::repository::Repository;

use anyhow::Result;
use crate::error;

pub fn reconstruct<P : AsRef<str>,>(pathspec: P, bare: bool) -> Result<()>  {
    
    // split up the the path into revision and the actual path
    let (rev, path) = parse_path_spec(pathspec.as_ref())?;

    // get the path relative to the repository root
    let path = Repository::get_path_relative_to_repo_here(path)?
        .to_string_lossy().into_owned();

    let path = if bare {
        path
    } else {
        // TODO: properly implement checking
        format!("{}.contents", path)
    };

    let data = Repository::reconstruct(&path, rev)?;

    // print it all to stdout
    let mut stdout = std::io::stdout();

    stdout.write_all(&data).and_then(|_| {
        if !data.ends_with(b"\n") {
            stdout.write_all(b"\n")
        } else {
            Ok( () )
        }
    }).expect("fatal - stdout error");
    
    Ok( () )
}


/// Parse the path specification in form of `rev:path`
fn parse_path_spec(pathspec: &str) -> Result<(&str, &str)> {
    use regex::Regex;
    
    let regex = Regex::new("^((?P<rev>[^:]*):)?(?P<path>.+)$").unwrap();

    let matches = regex.captures(pathspec).ok_or_else(|| {
        error::InvalidPathSpec {
            pathspec : pathspec.to_owned()
        }
    })?;

    let rev = matches.name("rev").map(|m| m.as_str()).unwrap_or("HEAD").trim();
    let path = matches.name("path").map(|m| m.as_str()).unwrap_or_default().trim();

    Ok( (rev, path) )
}