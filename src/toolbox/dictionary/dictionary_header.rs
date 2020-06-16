//
// src/toolbox/dictionary/dictionary_header.rs
//
// Toolbox dictionary header detection
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use crate::toolbox::Scanner;

impl<'a> Scanner<'a> {
    /// Expect a toolbox dictionary header
    ///
    /// Advances the scanner to the next non-blank line and returns an error
    /// if this lien is not a toolbox dictionary header. The error returned 
    /// is the number of the offending line in the file
    pub fn expect_toolbox_dictionary_header(mut self) -> Result<Self, usize> {
        use regex::Regex;
        use crate::toolbox::scanner::Token;

        // compile the toolbox dictionary regex
        // note: this could have been a global variable, but since this is not a performance-
        //       critical path, we can afford to recompile it again every time
        let re_header = Regex::new(
            r"^\\_sh[[:space:]]+v3\.0[[:space:]]+[0-9]+[[:space:]]+Dictionary[[:space:]]*$"
        ).expect("Internal regular expression error");

        // scan the file until we detect a toolbox dictionary header
        // abort on unexpected string
        let error_line = loop {
            match self.next() {
                // blank line
                Some( (_, Token::Blank )) => {
                    // continue scanning
                    continue;
                },
                // header line detected
                Some( (line, _) ) if re_header.is_match(line.text) => {
                    //  return success
                    return Ok( self );
                },
                // any other line
                Some( (line, _) ) => {
                    break line.line;
                }, 
                // end of file
                None => {
                    // it is correct to read last_line even if it was never properly set, 
                    // as we initialize it for the case the text is empty
                    break self.last_line.clone().line;
                }
            };
        };

        Err( error_line )
    } 

}