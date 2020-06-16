//
// src/toolbox/scanner.rs
//
// Utilities for parsing Toolbox files
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use arrayvec::ArrayVec;

/// A line in a text stream
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line<'a> {
    pub line : usize,
    pub text : &'a str,
}

/// A token that represents a basic structural elements of a toolbox file
#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
    /// Start of a new toolbox record (issued before the tagged text)
    RecordBegin,
    /// End of a toolbox record (with body)
    RecordEnd { body: &'a str },
    /// A tagged text line (tag contains the initial '\')
    Tagged {tag: &'a str, text: &'a str},
    /// An untagged text line
    Untagged {text: &'a str},
    /// A blank line (either empty or containing whitespaces only)
    Blank
}


/// A toolbox file scanner that interprets a toolbox file as a sequence of 
/// structural tokens
///
/// # Notes
///
/// The scanner has to emit multiple tokens for a single line in some circustances,
/// such as when a new record is encountered. A queue is used to store already
/// produced but not yet yielded tokens. Since the upper limit of tokens in the queue
/// is known, we use a statically allocated container. 
#[derive(Debug, Clone)]
pub struct Scanner<'a> {
    // the remaining text
    text        : &'a str,
    // the next to be scanned line number
    //
    // note: this field is unnessesary since we could have stored the counter                       
    //       inside the `last_line` field. That would make the logic slightly
    //       more complicated. We duplicate the information here for the sake
    //       of the implementation clarity
    next_line_i : usize,
    // the tag that marks a start of a new record
    record_tag  : String,
    // a queue used to map a single line to multiple tokens
    queue       : ArrayVec<[Token<'a>; 3]>,
    // the last scanned line
    pub(super) last_line  : Line<'a>,
    // marker for where the last record started
    start       : Option<&'a str>
}

impl<'a>  Scanner<'a> {
    pub fn from<S: Into<String>>(text: &'a str, record_tag: S) -> Scanner<'a> {
        Scanner {
            text,
            next_line_i : 0,
            record_tag  : record_tag.into(), 
            queue       : ArrayVec::new(),
            // the only case where this field can be read before it was 
            // "correctly" set is if the file is empty
            // setting last line to file contents in this case is correct
            last_line   : Line { line : 0, text }, 
            start       : None
        }   
    }
}

pub type ScannerItem<'a> = (Line<'a>, Token<'a>);

/// Iteration over a toolbox scanner returns a pair (line, token)
impl<'a>  Iterator for Scanner<'a> {
    type Item = ScannerItem<'a>;


    fn next(&mut self) -> Option<Self::Item> {
        use internal::*;

        // return tokens from the queue if it is not empty
        if let Some(token) = self.queue.pop() {
            return Some((self.last_line.clone(), token));
        };

        // guard against iteration end
        if self.text.is_empty() {
            // if there is an open record (start is not None), 
            // we must signal it's end
            // 
            // we put None in start so that it happens at most once
            return self.start.take().map(|start| {
                (
                    self.last_line.clone(), 
                    Token::RecordEnd { body : trim_trailing_empty_lines(start) }
                )
            });
        }

        // grab the next line
        let (line, tail) = {
            // split the text at the line end
            let end = self.text.find('\n').map_or(self.text.len(), |i| i+1);
            let (line, tail) = self.text.split_at(end);
            // remove the trailing end line markers from the line
            // TODO: there must be a better way of doing this
            (line.trim_end_matches(|c| c == '\r' || c == '\n'), tail)
        };

        // scan the line and produce the token
        let token = match ParsedLine::from(line) {
            // new record
            ParsedLine::Tagged(tag, text) if tag == self.record_tag => {
                // add the extra tokens to the queue
                self.queue.push(Token::Tagged { tag, text });
                self.queue.push(Token::RecordBegin);

                // save the record start
                // if this is not the first record, also
                // yield the last record body
                self.start.replace(self.text).iter().for_each(|start| {
                    let end = self.text.as_ptr() as usize - start.as_ptr() as usize;
                    let body = trim_trailing_empty_lines(&start[ .. end]);

                    self.queue.push(Token::RecordEnd { body });
                });

                // yield the top token
                self.queue.pop().unwrap()
            },
            // tagged line
            ParsedLine::Tagged(tag, text) => {
                Token::Tagged { tag, text }
            },           
            // untagged line
            ParsedLine::Untagged(text) => {
                Token::Untagged { text }
            },
            // blank line
            ParsedLine::Blank => {
                Token::Blank
            }
        };


        // set the remaining text to the tail
        self.text = tail;
        
        // save the line
        self.last_line = Line { line : self.next_line_i, text: line};

        // advance the next line counter
        self.next_line_i += 1;

        // yield the line number and the token, updating the line in the process
        Some( (self.last_line.clone(), token) )
    }
}


mod internal {
    /// Represents a line in a Toolbox file
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum ParsedLine<'a> {
      /// A tagged line: \tag value
      ///
      /// # Notes
      ///
      /// The tag includes the initial backslash
      Tagged(&'a str, &'a str),
      /// An untagged line
      Untagged(&'a str),
      /// Blank line 
      Blank
    }
    
    
    impl<'a> ParsedLine<'a> {
      /// Create a structured representation for a text line in a Toolbox file
      ///
      /// # Notes
      ///
      /// For tagged lines, the tag/value pair are continous string slices 
      /// that will yield `line` back when stiched together. The tag contains
      /// the initial backlash. The value contains all the spaces that follow
      /// the tag. That is, 
      ///
      ///    \tag value
      ///
      /// is parsed as "\tag", " value"
      ///
      /// For untagged lines, the `line` reference is simply copied. 
      pub fn from(line: &'a str) -> Self {
        use ParsedLine::*;

        match line {
          _ if line.starts_with('\\') => {
            // find where the tag end 
            // this is either the first whitespace
            // or the end of the line (if there is no value part)
            let end = line.find(char::is_whitespace).unwrap_or_else(|| line.len());
            // split the line into tag, value pair
            let (tag, value) = line.split_at(end);
    
            Tagged(tag, value)
          },
          _ if line.trim().is_empty() => {
            Blank
          },
          _ => {
            Untagged(line)
          }
        }
      }
    }

    /// Removes any trailing empty lines from a string slice
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(trim_trailing_empty_lines("test1"), "test1");
    /// assert_eq!(trim_trailing_empty_lines("test1\n"), "test1\n");
    /// assert_eq!(trim_trailing_empty_lines("test1\r\n"), "test1\r\n");
    /// assert_eq!(trim_trailing_empty_lines("test1\n\n"), "test1\n");
    /// assert_eq!(trim_trailing_empty_lines("test1\r\n\r\n"), "test1\r\n");
    /// ```
    pub fn trim_trailing_empty_lines(text: &str) -> &str {
        let lines = text.rsplit_terminator('\n');
    
        // skip all the empty lines
        let mut end = text.len();
        for line in lines {
            if line.trim().is_empty() {
                end = line.as_ptr() as usize - text.as_ptr() as usize;
            } else {
                break
            }
        }
        
        &text[ .. end]
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_line() {
        use super::internal::ParsedLine;
        use super::internal::ParsedLine::*;

        assert_eq!(ParsedLine::from(r"\tag value")     , Tagged(r"\tag", r" value"));
        assert_eq!(ParsedLine::from(r"\tag   value  ") , Tagged(r"\tag", r"   value  "));
        assert_eq!(ParsedLine::from(r"value")          , Untagged(r"value"));
        assert_eq!(ParsedLine::from(r"  value  ")      , Untagged(r"  value  "));
        assert_eq!(ParsedLine::from(r"    ")           , Blank);
    }

    #[test]
    fn test_trim_trailing_empty_lines() {
        use super::internal::trim_trailing_empty_lines;

        assert_eq!(trim_trailing_empty_lines(""), "");
        assert_eq!(trim_trailing_empty_lines("test1"), "test1");
        assert_eq!(trim_trailing_empty_lines("test1\n"), "test1\n");
        assert_eq!(trim_trailing_empty_lines("test1\r\n"), "test1\r\n");
        assert_eq!(trim_trailing_empty_lines("test1\n\n"), "test1\n");
        assert_eq!(trim_trailing_empty_lines("test1\r\n\r\n"), "test1\r\n");
    }
}




