//
// src/config.rs 
//
// git-toolbox configuration representation and TOML parsing
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


/// Name of the git-toolbox configuration file
pub const CONFIG_FILE : &str = "git-toolbox.toml";


use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, smart_default::SmartDefault)]
#[serde(rename_all="lowercase")]
pub enum UserRole {
    #[default]
    User,
    Manager
}

#[derive(Deserialize, Debug, Clone)]
pub struct UserConfig {
    pub name: String,
    #[serde(default)]
    pub role: UserRole,
    pub namespace: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all="kebab-case")]
pub struct DictionaryConfig {
    pub name: String,
    pub path: String,
    #[serde(deserialize_with = "deserialize::read_marker")]
    pub record_tag: String,
    #[serde(default)]
    pub unique_id : bool,
    #[serde(default, deserialize_with = "deserialize::read_marker_option")]
    pub id_tag    : Option<String>,
    #[serde(
        default = "deserialize::default_id_spec", deserialize_with = "deserialize::read_regex_option"
    )]
    pub id_spec   : regex::Regex,
    #[serde(default)]
    pub lifecycle : bool,
    #[serde(default, deserialize_with = "deserialize::read_marker_option")]
    pub lifecycle_tag : Option<String>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(rename = "user", default)]
    pub users: Vec<UserConfig>,
    #[serde(rename = "dictionary", default)]
    pub dictionaries: Vec<DictionaryConfig>,
}


// 
// ####                    ###  
//  ##                      ##  
//  ##                      ##  
//  ##  ## ##  ##   ## ##   ##  
//  ##  ### ### ##  ### ##  ##  
//  ##  ##  ##  ##  ##  ##  ##  
//  ##  ##  ##  ##  ##  ##  ##  
// #### ##  ##  ##  #####  #### 
//                  ##
//                 ####

impl Config {
    /// Locate the dictionary config by path
    ///
    /// Path is assumed to be relative to the repository
    pub fn dictionary_by_path<P: AsRef<str>>(&self, path: P) -> anyhow::Result<&DictionaryConfig> {
        use crate::error;
        use anyhow::bail;

        let matched_dictionary = self.dictionaries.iter().filter(|cfg| {
            cfg.path == path.as_ref()
        }).collect::<Vec<_>>();

        if matched_dictionary.len() != 1 {
            bail!(
                error::NotAManagedFile {
                    path : path.as_ref().to_owned().into()
                }
            );
        };

        Ok( matched_dictionary[0] )
    }
}


mod deserialize {
    use anyhow::Result;
    use crate::error;
    use super::Config;
    
    impl std::convert::TryFrom<&[u8]> for Config {
        type Error = anyhow::Error;
    
        fn try_from(blob: &[u8]) -> Result<Config> {  
            // convert the blob into utf-8 encoded text
            let text = std::str::from_utf8(blob).map_err(|err| {
                error::ConfigurationError {
                    text : String::new(),
                    at   : None, 
                    msg  : err.to_string()
                }
            })?;
    
            // parse the toml file
            toml::from_str(text).map_err(|err| {
                error::ConfigurationError {
                    text : text.to_owned(),
                    at   : err.line_col(),
                    msg  : err.to_string()
                }
                // convert this into anyhow::Error
                .into()
            })
        }
    }
    
    
    use serde::{Deserialize, Deserializer};
    
    pub fn read_marker<'a, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'a>,
    {
        // read the basic string
        let s: &str = Deserialize::deserialize(deserializer)?;
      
        // add the prefix
        Ok( r"\".to_owned() + s )
    }
    
    
    pub fn read_marker_option<'a, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'a>,
    {
        // read the basic string
        read_marker(deserializer).map(Some)
    }
    
    
    pub fn read_regex_option<'a, D>(deserializer: D) -> Result<regex::Regex, D::Error>
    where
        D: Deserializer<'a>,
    {
        use serde::de::Error;
        use std::collections::HashSet;
    
        // read the basic string
        let re: &str = Deserialize::deserialize(deserializer)?;
    
        let re = regex::Regex::new(re).map_err(|e| {
            Error::custom(e)
        })?;
    
        // validate the capture names
        let capture_names : HashSet<_> = re.capture_names().collect();
    
        if !capture_names.contains(&Some("namespace")) {
            return Err( 
                Error::custom("ID regex has to contain the group (?P<namespace>...)") 
            );
        }
    
        if !capture_names.contains(&Some("id")) {
            return Err( 
                Error::custom("ID regex has to contain the group (?P<namespace>...)")
            );
        }
    
    
        Ok( re )  
    }
    
    
    pub fn default_id_spec() -> regex::Regex {
        regex::Regex::new("$(?P<id>.+)^").expect("Internal error - invalid regex")
    }
}