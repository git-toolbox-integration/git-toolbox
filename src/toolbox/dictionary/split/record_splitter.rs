//
// src/toolbox/dictionary/split/id_splitter.rs
//
// Splitter that handles dictionaries without unique IDs
//
// Produces one CLOB per record label (potentially multiple records per CLOB)
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use crate::toolbox::Dictionary;
use crate::toolbox::scanner::*;

use super::SplitterOutput;

/// A basic toolbox dictionary splitter (no uniqiue identifiers or lifecycle management)
pub fn split(dictionary: Dictionary) -> SplitterOutput {
    use crate::repository::Clob;
    use crate::toolbox::ToolboxFileIssue;
    use multimap::MultiMap;

    use crate::util::*;
  
    // deconstruct the dictionary
    let mut scanner = dictionary.scanner;
    let config  = dictionary.config;
    let mut issues = dictionary.issues;

    // report any lines orphaned before the first record
    let mut orphaned_lines = vec!();

    scanner.try_for_each(|token| {
        use Token::*;

        match token {
            // record start - quit the initial scan
            (_, RecordBegin) => {
                return None
            },
            (line, Tagged { tag: _, text: _}) | (line, Untagged { text: _ }) => {
                issues.push(
                    ToolboxFileIssue::LineBeforeFirstRecord {
                        line: line.clone()
                    }
                );

                orphaned_lines.push(line.text);
            }, 
            (_, Blank) => {
                // push an empty line if it does not create lare blanks of space
                if orphaned_lines.last().map(|line| !line.trim().is_empty()).unwrap_or(false) {
                    orphaned_lines.push(""); 
                }
            }
            _ => {
            }
        }

        Some( () )
    });


    let mut clobs = MultiMap::new();
    
    // current record label
    let mut record_label = String::new();
    
    for token in scanner {
        use Token::*;

        match token {
            // record start tag
            (line, Tagged {tag, text}) if tag == config.record_tag => {
                // remove the trailing spaces
                let text = text.trim();
                if text.is_empty() {
                    issues.push(
                        ToolboxFileIssue::MissingRecordLabel { 
                            line
                        }
                    )    
                }

                // use the acii-only sanitized label
                record_label = sanitize_label(text.trim());
            },
            // untagged line
            (line, Untagged {text:_}) => {
                issues.push(
                    ToolboxFileIssue::UntaggedLine {
                        line: line.clone()
                    }
                )
            },
            // record end — add new record
            (_, RecordEnd { body }) => {
                clobs.insert(std::mem::take(&mut record_label), body);
            },
            _ => {
            }
        }
    };


    let result = clobs.into_iter().map(move |(label, records)| {
        // build a path for the record
        let path = if label.is_empty() {
            "invalid/label_missing.txt".to_owned()
        } else {
            format!("{}/{}.txt", build_path_prefix(&label), &label)
        };

        // build the clob contents by joining the records 
        // together
        // TODO: do we sort the records somehow?
        let content = records.join("\n");
    
        Clob { path, content }
     })
    // add the orphaned lines
    .chain({
        std::iter::once(orphaned_lines.join("\n")).map(|mut text| {
            // add line end (if nessesary)
            if !text.ends_with('\n') {
                text.push('\n')
            }

            text
        })
        // ignore the orphaned lines block if it is empty
        .filter(|text| {
            !text.trim().is_empty()
        })
        // make it into a clob
        .map(|content| {
            Clob { path: "invalid/__.txt".to_owned(), content }
        })
    });

    
    ( Box::new(result.map(Clob::validated)), issues )
}
