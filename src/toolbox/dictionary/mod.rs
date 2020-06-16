//
// src/toolbox/dictionary
//
// A Toolbox dictionary parser and splitter
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

// the dictionary parser implementation
mod dictionary_impl;
mod dictionary_header;

// dictionary splitting
mod split;

pub use dictionary_impl::Dictionary;
