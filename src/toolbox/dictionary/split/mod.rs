//
// src/toolbox/dictionary/split
//
// Routines that parse a toolbox dictionary and split out a list of 
// toolbox records as well as detected issues
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use crate::repository::Clob;
use crate::toolbox::ToolboxFileIssue;

type SplitterOutput = (Box<dyn Iterator<Item=Clob> + 'static>, Vec<ToolboxFileIssue>);

use super::Dictionary;

mod record_splitter;
mod id_splitter;


impl Dictionary {
    pub fn split(self) -> SplitterOutput {
        // lifecycle-managed dictionary
        if self.config.lifecycle {
            panic!("Lifecycle dictionaries are not yet implemented")
        } 
        // id-managed dictionary
        else if self.config.unique_id { 
            id_splitter::split(self)
        } else {
            record_splitter::split(self)
        }
    }    
}

