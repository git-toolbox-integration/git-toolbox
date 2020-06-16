//
// src/toolbox/issue.rs
//
// All issues that we can detect in Toolbox files are defined here. 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use super::scanner::Line;

/// An error in a toolbox file's contents
#[derive(Debug, PartialEq, Eq)]
pub enum ToolboxFileIssue {
    /// Content occurs before the first record
    LineBeforeFirstRecord { 
        line: Line<'static>
    }, 
    /// Untagged line in a dictionary file
    UntaggedLine { 
        line: Line<'static> 
    }, 
    /// Record without a label
    MissingRecordLabel { 
        line : Line<'static> 
    }, 
    /// Missing ID
    MissingID { 
        line : Line<'static> 
    },
    /// Invalid ID
    InvalidID { 
        record : Line<'static>,
        line   : Line<'static>
    },
    /// Multiple IDs per record
    ExtraneousID {
        record : Line<'static>,
        line   : Line<'static>  
    },
    /// Ambiguous ID (same id found in multiple records)
    AmbiguousID {
        record : Line<'static>,
        line   : Line<'static>  
    },
    /// Missing dictionary header
    MissingDictionaryHeader {
        line : usize
    }
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

use std::fmt;



impl ToolboxFileIssue {
    pub fn line(&self) -> usize {
        match self {
            ToolboxFileIssue::LineBeforeFirstRecord { line }   |
            ToolboxFileIssue::UntaggedLine { line }            |
            ToolboxFileIssue::MissingRecordLabel { line }      |
            ToolboxFileIssue::MissingID { line }               |
            ToolboxFileIssue::InvalidID { record : _, line }   |  
            ToolboxFileIssue::ExtraneousID { record : _, line} |
            ToolboxFileIssue::AmbiguousID { record : _, line }  => {
                line.line
            },
            ToolboxFileIssue::MissingDictionaryHeader { line } => {
                *line
            }
        }
    }
}

impl std::error::Error for ToolboxFileIssue {}

impl fmt::Display for ToolboxFileIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use crate::util::truncate_text;
        use style::*;

        // build the error message
        let message = match self {
            ToolboxFileIssue::LineBeforeFirstRecord { line } => {
                format!(
                    "{} line {} occurs before the first record",
                    header(line.line),
                    value(truncate_text(line.text, 30))
                )
            },
            ToolboxFileIssue::UntaggedLine { line } => {
                format!(
                    "{} untagged line {}",
                    header(line.line),
                    value(truncate_text(line.text, 30))
                )
            },
            ToolboxFileIssue::MissingRecordLabel { line } => {
                format!(
                    "{} missing a label in the record {}",
                    header(line.line),
                    value(line.text.trim())
                )
            },
            ToolboxFileIssue::MissingID { line } => {
                format!(
                    "{} missing ID tag in the record {}",
                    header(line.line),
                    value(line.text.trim())
                )
            },
            ToolboxFileIssue::InvalidID { record, line } => {
                format!(
                    "{} invalid ID tag {} in the record {}",
                    header(line.line),
                    value(line.text.trim()),
                    value(record.text.trim())
                )
            }, 
            ToolboxFileIssue::ExtraneousID { record, line } => {
                format!(
                    "{} extraneous ID tag {} will be ingored in the record {}",
                    header(line.line),
                    value(line.text.trim()),
                    value(record.text.trim())
                )
            }, 
            ToolboxFileIssue::AmbiguousID { record, line } => {
                format!(
                    "{} ID tag {} in the record {} is not unique",
                    header(line.line),
                    value(line.text.trim()),
                    value(record.text.trim())
                )
            },
            ToolboxFileIssue::MissingDictionaryHeader { line } => {
                format!(
                    "{} Missing Toolbox dictionary header",
                    header(*line)
                )  
            }
        };

        // and write it
        write!(formatter, "{}", message)
    }
}


mod style {
    use std::fmt::Display;
    use console::Style;

    fn basic_style() -> Style {
        Style::new().force_styling(true)
    }

    pub fn value<D: Display>(obj: D) -> impl Display {
        format!("'{}'", basic_style().cyan().apply_to(obj))
    }


    pub fn header(line: usize) -> impl Display {
        basic_style().italic().yellow().apply_to(format!("line:{:<8}", line+1))
    }
}






 



