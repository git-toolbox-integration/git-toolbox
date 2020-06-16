//
// git-toolbox 
//
// A git extension for Field Linguist's Toolbox
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0


// Errors
#[macro_use] mod error_macros;
mod error;

// CLI interface
#[macro_use] extern crate clap;
#[macro_use] mod cli_app;

// Various internal frameworks and utilities
mod config;
mod repository;
mod toolbox;
mod listing_formatter;
mod util;

// Implementation of CLI commands

// git-toolbox setup
mod setup;
// git-toolbox status
mod status;
// git-toolbox gitfilter
mod git_filter;
// git-toolbox show
mod reconstruct;
// git-toolbox stage
mod stage;
// git-toolbox reset
mod reset;

// Program's entry point
fn main() {
    use cli_app::Command;

    // fetch and run the command from CLI
    let result = Command::from_cli().and_then(|command| {
        match command {
            Command::Setup { init } => {
                setup::setup(init)
            }, 
            Command::Reset { files, verbose, force} => {
                reset::reset(files, verbose, force)
            },
            Command::Stage { files, verbose, discard_workdir_changes} => {
                stage::stage(files, verbose, discard_workdir_changes)
            },
            Command::Status { files, verbose } => {
                status::status(files, verbose)
            }, 
            Command::Reconstruct { pathspec, bare} => {
                reconstruct::reconstruct(pathspec, bare)
            },            
            Command::FilterClean { path } => {
                git_filter::clean(path)
            },
            Command::FilterSmudge { path } => {
                reconstruct::reconstruct(path, false)
            }
        }
    });

    // check if there was an error, display it and die
    if let Err(err) = result {
        stderr!("{}", err);
        std::process::exit(1);
    }
}
