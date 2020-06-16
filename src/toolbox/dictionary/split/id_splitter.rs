//
// src/toolbox/dictionary/split/id_splitter.rs
//
// Splitter that handles dictionaries with unique IDs
//
// Produces one CLOB per record (records with invalid id's are 
// collected in a dedicated CLOB)
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use crate::toolbox::Dictionary;
use crate::toolbox::scanner::*;
use crate::toolbox::ToolboxFileIssue;

use super::SplitterOutput;


#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct ID<'a> {
    full      : &'a str,
    namespace : Option<&'a str>,
    id        : &'a str
}

fn extract_id<'a>(text : &'a str, regex: &regex::Regex) -> Result<ID<'a>, ()> {
    // use the regex to match the id 
    let captures = regex.captures(text)
        // check that the entire text was matched 
        .filter(|captures| {
            captures.get(0).expect("Internal error: invalid ID regex").as_str() == text 
        })
        // turn it into a result<ID>
        .ok_or_else(|| () )?;

    // extract the namespace component
    let namespace = captures.name("namespace")
        .map(|val| val.as_str().trim())
        .filter(|val| !val.is_empty());

    // extract the id compoentn
    let id = captures.name("id").expect("Internal error: invalid ID regex").as_str().trim();

    // final validation and ID construction
    if id.is_empty() {
        Err( () )
    } else {
        Ok( ID { full: text, namespace, id } )
    }
}

/// A basic toolbox dictionary splitter (no uniqiue identifiers or lifecycle management)
pub fn split(dictionary: Dictionary) -> SplitterOutput {
    use crate::repository::Clob;
    use multimap::MultiMap;
    use itertools::Itertools;

    use crate::util::*;

    // decosntruct the dictionary
    let mut scanner = dictionary.scanner;
    let config  = dictionary.config;
    let mut issues = dictionary.issues;

    // cache the id tag 
    let id_tag = config.id_tag.as_ref().expect("Internal error: wrong splitting algorithm");
  
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

    // a map from IDs to records
    // 
    // ID -> (first record line, id line, record contents)
    let mut id_map = MultiMap::new();

    // list of records that do not have ids
    let mut id_missing = vec!();

    // current record label
    let mut record_start   = Line { line : 0, text : "" };
    let mut record_id_line = Line { line : 0, text : "" };
    let mut record_id      = None; 
    

    for token in scanner {
        use Token::*;

        match token {
            // record start tag
            (line, Tagged {tag, text}) if tag == config.record_tag => {
                record_start = line.clone();
                if text.trim().is_empty() {
                    issues.push(
                        ToolboxFileIssue::MissingRecordLabel { 
                            line
                        }
                    )    
                }
            },
            // record id tag
            (line, Tagged {tag, text}) if tag == id_tag => {
                // check if this is the first id spec for this line
                if record_id.is_some() {
                    issues.push(
                        ToolboxFileIssue::ExtraneousID {
                            record : record_start.clone(),
                            line   : line.clone(),    
                        }
                    )
                };

                // remove the exess whitespace
                let text = text.trim();

                // extract and store the id, reporting issues (if any)
                let _ = extract_id(text, &config.id_spec).map(|id| {
                    if record_id.is_none() {
                        record_id.replace(id);
                        record_id_line = line.clone();
                    }
                }).map_err(|_| {
                    issues.push(
                        ToolboxFileIssue::InvalidID {
                            record : record_start.clone(),
                            line   : line.clone(),
                        }
                    )
                });
            },
            // untagged line
            (line, Untagged {text: _}) => {
                issues.push(
                    ToolboxFileIssue::UntaggedLine {
                        line: line.clone()
                    }
                )
            },
            // record end — add new record
            (_, RecordEnd { body }) => {
                if let Some(id) = record_id.take() {  
                    // record this id occurence
                    id_map.insert(id.clone(), (record_start.clone(), record_id_line.clone(), body));
                } else {
                    // this record does not have an ID which make 
                    id_missing.push(body);

                    // report the problem
                    issues.push(
                        ToolboxFileIssue::MissingID {
                            line: record_start.clone()
                        }
                    );
                }
            },
            _ => {
            }
        }
    };

    // detect and report the ambiguous IDs
    for (_, records) in id_map.iter_all().filter(|(_,v)| v.len()>1) {
        for (record, line, _) in records.iter() { 
            issues.push(
                ToolboxFileIssue::AmbiguousID {
                    record : record.clone(), 
                    line   : line.clone()
                }
            );    
        }
    }

    // sort the issues
    issues.sort_unstable_by_key(|issue| issue.line());

    // construct the result iterator
    let result = id_map.into_iter().map(move |(id, records)| {
        // build a path for the record
        let path = if let Some(ns) = id.namespace {
            format!("private/{}/{}.txt", ns, &id.full)
        } else {
            format!("public/{}/{}.txt", build_path_prefix(&id.id), &id.full)
        };

        // build the clob contents by joining the records 
        // together
        // TODO: do we sort the records somehow?
        let content = records.into_iter().map(|(_, _, body)| body).join("\n");
    
        Clob { path, content }
     })
    // add the id_missing records
    .chain({
        std::iter::once(id_missing.join("\n")).map(|content| {
            Clob { path: "invalid/id_missing.txt".to_owned(), content }
        })
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
    })
    .map(Clob::validated);

    ( Box::new(result.map(Clob::validated)), issues )
}
