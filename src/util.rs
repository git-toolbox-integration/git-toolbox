//
// src/util.rs 
//
// Various utilities such as path and text manipulation. Among others, 
// `build_path_prefix()` (the utility for spearing out files in a fs tree) 
// is defined here.
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


use std::path::Path;

use anyhow::Result;

/// Reads a file into memory with static lifetime
///
/// The text is leaked to simplify lifetime management when workign with &str. 
/// Since we need to load the entire toolbox file into memory anyway, and the tool
/// only loads few of them during invocation, this is something we can afford to do
pub fn _read_file_to_str<P: AsRef<Path>>(path: P) -> Result<Option<&'static str>> {
  use anyhow::Context;

  let path = path.as_ref();

  // read the file
  _read_file(path).and_then(|maybe_blob| {
    maybe_blob.map(|blob| -> Result<&'static str> {
        // convert the blob to an UTF-8 encoded string and leak the memory
        Ok( Box::leak(String::from_utf8(blob)?.into_boxed_str()) )
    })
    .transpose()
    .with_context(|| format!("Invalid UTF-8 sequence in '{}'", path.display()))
  })
}  

// Reads a text file that may not exost
pub fn _read_file<P: AsRef<Path>>(path: P) -> Result<Option<Vec<u8>>> {
    use std::fs;
    use std::io;
    use anyhow::Context;

    let path = path.as_ref();

    fs::read(path)
        .map(Some)
        .or_else(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                Ok( None )
            } else {
                Err ( err )
            }
        })
        .with_context(|| {
            format!("Error reading '{}'", path.display())
        })
}

/// Sanitizes a label, making sure it can be used as a cross-platform 
/// file name
///
/// This will translate unicode glyphs to ascii sequences and replace
/// punctuation and other symbols
///
/// # Notes
///
/// It is possible for two labels that compare as not equal to produce
/// equal sanitized strings
pub fn sanitize_label(label: &str) -> String {
    use deunicode::AsciiChars;

    let sanitized = label.ascii_chars()
        .map(|chars| chars.unwrap_or("_").chars())
        .flatten()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .fold(String::new(), |mut buff, c| {
            if !(c == '_' && buff.ends_with('_')) {
                buff.push(c);
            } 

            buff
        });

    assert!(&sanitized.is_ascii(), "Non-ASCII characters in sanitized label '{}'!", sanitized);

    sanitized
}

/// Generate a nested path prefix for a name
///
/// This function will construct a path from the first four characters 
/// of the `name`, after unicode normalization and removal of all non-letter-like
/// unicode components. If the name is too short, the prefix will be appended with
/// the undesrore sign '_' in order to produce uniform-sized paths
///
/// This function is used to spead files over multiple directories to prevent
/// directory overcrowding. All the directories have transparent names, which 
/// allow a human user to easily find a file by its name. 
///
/// # Notes
///
/// This functionality is usually achieved by gegerating a prefix from a
/// hash. This ensures more balanced distribution of files at the cost of
/// discoverability. Since we want the users to be able to navigate to 
/// a specific file quickly, we don't use hashes (and have to live with 
/// the fact that some directories will have more files in them)
pub fn build_path_prefix(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    use itertools::Itertools;
    use std::iter;
  
    // extract a four letter prefix from the name
    let prefixes = name
        // use canonical decomposition
        // to split up and eliminate combining marks etc.
        // leaving only base letter-like components 
        .nfd()
        .filter(|c| { c.is_alphanumeric() })
        // limit the sequence to 4 letter-likes
        .take(4)
        // extend with sequence of _ in case the prefix itself
        // is too short
        .chain(iter::repeat_with(|| '_'))
        // limit the sequence to 4 letter-likes
        .take(4)
        // consume the prefixes in chunks of two 
        .chunks(2);
    
    // join the prefixes
    prefixes.into_iter().fold(String::new(), |mut accum, s| {
        if !accum.is_empty() { accum.push('/'); } 
        accum.extend(s);

        accum
  })
}


/// Truncate the text to the given display length, adding ellipsis dots if truncated
pub fn truncate_text(text: &str, length : usize) -> String {
  use unicode_segmentation::UnicodeSegmentation;

  let mut result = String::with_capacity(length + 3);

  for grapheme in text.graphemes(true).take(length.saturating_sub(3)) {
    result.push_str(grapheme);
  }
  if result.len() < text.len() {
    result.push_str("...");
  }

  result
}

/// Obtain the path relative to the current directory
pub fn get_relative_path<P: AsRef<std::path::Path>>(path: P) -> std::path::PathBuf {
    use pathdiff::diff_paths;

    std::env::current_dir().map_or_else(
        // if current dir cannot be retrieved, just return the original
        |_| path.as_ref().into(),
        |current| diff_paths(path.as_ref(), current).unwrap_or_else(|| path.as_ref().into())
    )
}

/// Obtain the absolute path
pub fn absolute_path<P: AsRef<std::path::Path>>(path: P) -> std::path::PathBuf {
    use path_clean::PathClean;

    let path = path.as_ref();

    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().expect("fatal: unable to get the current directory").join(path)
    }.clean()
}


/// Escape a string sing ANSI-C rules
///
/// If the string does not need escaping, it is returned unchanged
/// This differs from `escape_default()` in that it does not escape
/// unicode characters
pub fn c_escape_str<S: AsRef<str>>(string: S) -> String {
  let mut escaped = String::new(); 

  escaped.push('"');

  for c in string.as_ref().chars() {
    if c.is_ascii() {
      escaped.extend(c.escape_default());
    } else {
      escaped.push(c);
    }
  }

  escaped.push('"');

  escaped
}


/// Escape unicode characters as \u sequences
pub fn escape_unicode_only(s: &str) -> String {
    s.chars().fold(String::new(), |mut buf, ch| {
        if ch.is_ascii() && ! ch.is_control() {
            buf.push(ch);
        } else {
            buf.extend(ch.escape_unicode());
        }

        buf
    })
}


