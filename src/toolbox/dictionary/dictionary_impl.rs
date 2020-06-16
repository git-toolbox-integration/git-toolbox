//
// src/toolbox/dictionary/dictionary_impl.rs
//
// A Toolbox dictionary loader 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use crate::config::DictionaryConfig;
use crate::repository::Repository;
use crate::toolbox::{Scanner, ToolboxFileIssue};

use anyhow::Result;
use crate::error;

/// A Toolbox dictionary
#[derive(Debug)]
pub struct Dictionary {
    pub(super) config  : DictionaryConfig,
    pub(super) text    : &'static str,
    pub(super) scanner : Scanner<'static>,
    pub(super) issues  : Vec<ToolboxFileIssue>
}

impl Dictionary {
    pub fn load(repo: &Repository, config: &DictionaryConfig, strict: bool) -> Result<Dictionary> {
        use std::fs;

        let config = config.clone();

        let path = repo.workdir()?.to_owned().join(&config.path);
        let mut issues = vec!();

        // load the dictionary text 
        // we leak the memory here to simplify lifetime handling
        // this is not a problem since the tool only loads a dictionary once
        let text : &'static str = fs::read_to_string(&path)
            // leak the string
            .map(|text| Box::leak(text.into_boxed_str()))
            // process the errors
            .map_err(|err| -> anyhow::Error {
                use std::io::ErrorKind;

                //let path : std::path::PathBuf = config.path.clone().into();

                match err.kind() {
                    ErrorKind::NotFound    => {
                        error::FileNotFound { 
                            path: path.clone() 
                        }.into()
                    }
                    _                      => {
                        error::FileReadError {
                            path : path.clone(),
                            msg  : err.to_string()
                        }.into()   
                    }
                }
            })?;


        // start the toolbox scanner and check that the file has a dictionary header
        // if we are in the strict mode, we want to flag missign header as an error
        // in the non-strict mode, we tolerate the absence of the header 
        let scanner = Scanner::from(text, &config.record_tag)
            .expect_toolbox_dictionary_header()
            .or_else(|line| {
                if strict {
                    // return an error
                    Err(
                        error::ToolboxDictionaryMissingHeader {
                            path : path.clone(), 
                            text, 
                            line
                        }
                    )
                } else {
                    // simply reset the scanner
                    issues.push(ToolboxFileIssue::MissingDictionaryHeader { line });
                    
                    Ok( Scanner::from(text, &config.record_tag) )
                }
            })?;

        Ok (
            Dictionary {
                config, 
                text, 
                scanner,
                issues
            }
        )
    }

    pub fn _config(&self) -> &DictionaryConfig {
        &self.config
    }

    pub fn contents_root(&self) -> String {
        format!("{}.contents", &self.config.path)
    }
} 