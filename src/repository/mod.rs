//
// src/toolbox/repository
//
// Repository management.
//
// Implements API for manipulating the git repository, file I/O, etc. 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

pub const MANAGED_FILE_TEXT : &str = concat!(
    "This file is managed by git-toolbox.\n",
    "\n",
    "If you see this text, your repository is either misconfigured or has encountered\n",
    "an error during operation. Please run \"git toolbox reset\" and contact IT support\n", 
    "if your issue persists.\n"
);


// basic git wrapper
mod repo;
// repository configuration (setting git config etc.)
mod config;
// compute diffs between file contents
mod diff;
// abstraction over git index manipulation
mod staging_area;
// reconstructing managed file contents
mod reconstruct;


pub use diff::{Clob, ClobDiff, ClobValidationIssue, DiffStats};
pub use repo::Repository;

