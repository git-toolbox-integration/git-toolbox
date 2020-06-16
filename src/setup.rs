//
// src/setup.rs 
//
// Implementation of git-toolbox setup 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use crate::repository::Repository;
use crate::config::CONFIG_FILE;
use crate::cli_app::style;

use anyhow::{Result, anyhow, bail};
use crate::error;

// stub config file
const CONFIG_FILE_EXAMPLE : &str = r#"
# This is an example file, please edit me!

[[dictionary]]
name       = "Test Lexical Dictionary"
path       = "dictionaries/LexicalDic.txt"
record-tag = "lex"

# this dictionary uses unique IDs
# the regular expression allows the IDs to be validated and broken down
# see the manual for explanation
unique-id = true
id-tag    = "id"
id-spec   = "(?P<namespace>[a-zA-Z]*)(?P<id>[0-9]+)" 



[[dictionary]]
name = "Test Parsing Dictionary"
path = "dictionaries/ParsingDic.txt"
record-tag = "lex"
"#;

pub fn setup(init: bool) -> Result<()> {
    // init flag is set, we want to create an example config file
    if init {
        let config_path = Repository::workdir_for_repo_here()?.join(CONFIG_FILE);

        if config_path.exists() {
            bail!(error::ConfigurationExists)
        }

        std::fs::write(&config_path, &CONFIG_FILE_EXAMPLE).map_err(|err| {
            error::FileWriteError {
                path : config_path,
                msg  : err.to_string()
            }
        })?;

        stdout!("\n✅  Written a sample configuration file. Please edit it and run \"{}\" again", 
            cmd = style("git toolbox setup").bold()
        );

        return Ok( () );
    }

    // run the repository configuration
    Repository::configure().map_err(|err| {
        // update the error message
        anyhow!(
            "{err}\n\n⚠️  There were errors. Configuration might be incomplete.",
            err = err
        )
    })?;

    stdout!("\n✅  Configuration succesfully updated");
    Ok( () )
}