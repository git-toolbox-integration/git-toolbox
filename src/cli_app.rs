//
// src/cli_app.rs 
//
// CLI interfacing, command-line argument parsing, standard output macros. 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use clap::App;

/// CLI command parser using Clap
fn clap_app_spec<'a, 'b>() -> App<'a, 'b> {
    clap_app!( ("git-toolbox") =>
        (author: "Taras Zakharko <taras.zakharko@uzh.ch>")
        (about: "Git support for Linguist's Toolbox")
        (@arg verbose: -v "Verbose output")
        (@setting SubcommandRequired)
        (@subcommand gitfilter => 
            (@setting Hidden)
            (@group filter +required => 
                (@arg clean: --clean <FILE> !required)
                (@arg smudge: --smudge <FILE> !required)
            )
        )
        (@subcommand setup =>
            (about: "updates the repository configuration according to the configuration file")
            (@arg verbose: -v "Verbose output")
            (@arg init: --init "Create a sample configuration")
        )
        (@subcommand stage =>
            (about: "adds the changes in the managed toolbox files to the git staged area")
            (@arg FILES: ... !required 
                    "the managed file to stage (if not provided, all files will be staged)"
            )
            (@arg verbose: -v "Verbose output")
            (@arg ("discard-external-changes"): --("discard-external-changes")
                "overwrite external changes to the managed files if nessesary"
            )
        )
        (@subcommand reset =>
            (about: "discards the changes in the managed toolbox files (analogue to git reset)")
            (@arg FILES: ... !required 
                "the managed file to reset (if not provided, all files will be reset)"
            )
            (@arg verbose: -v "Verbose output")
            (@arg force: -f --force "Force reset")
        )
        (@subcommand status =>
            (about: "prints the information about the status of the managed toolbox files")
            (@arg verbose: -v "Verbose output")   
        )        
        (@subcommand show =>
            (about: "Prints the reconstituted contents of a managed toolbox file")
            (@arg PATHSPEC: +required 
                "git pathspec of to a managed file. Contents is fetched from HEAD unless \
                another git revision is specified (e.g. 'HEAD~1:path')"
            )
            (@arg bare: -n --bare
                "the path is a contents directory path, not a managed file path"
            )   
        )
    )
}


/// Git toolbox command
#[derive(Clone, Debug)]
pub enum Command {
    /// git-toolbox setup
    Setup {
        init: bool
    },
    /// git-toolbox status
    Status {
        files: Vec<String>,
        verbose: bool
    },
    /// git-toolbox stage
    Stage {
        files: Vec<String>,
        verbose: bool,
        discard_workdir_changes: bool
    },
    /// git-toolbox reset
    Reset {
        files: Vec<String>,
        verbose: bool,
        force: bool
    },
    /// git-toolbox gitfilter --clean
    FilterClean {
        path  : String  
    },
    /// git-toolbox gitfilter --smudge
    FilterSmudge {
        path  : String  
    },
    /// git-toolbox gitfilter show
    Reconstruct {
        pathspec : String, 
        bare : bool
    },
}

/// ANSI-terminal styling wrapper
pub fn style<D: std::fmt::Display>(obj: D) -> console::StyledObject<D> {
    console::Style::new().force_styling(true).apply_to(obj)
}


macro_rules! stdout {
    ($fmt:expr) => {
        stdout!("{}", $fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {{
        if ::console::colors_enabled() {
            println!($fmt, $($arg)*);
        } else {
            println!("{}", ::console::strip_ansi_codes(&format!($fmt, $($arg)*)));
        }
    }}    
}

macro_rules! stderr {
    ($fmt:expr) => {
        stderr!("{}", $fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {{
        if ::console::colors_enabled() {
            eprintln!($fmt, $($arg)*);
        } else {
            eprintln!("{}", ::console::strip_ansi_codes(&format!($fmt, $($arg)*)));
        }
    }}    
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

use anyhow::Result;

impl Command {
    pub fn from_cli() -> Result<Self> {
        let args = clap_app_spec().get_matches_safe()?;

        let verbose = args.is_present("verbose");

        let command = match args.subcommand() {
            ("setup", Some(cmd)) => {
                Command::Setup {
                    init : cmd.is_present("init")
                }
            },
            ("status", Some(cmd)) => {
                Command::Status {
                    files   : cmd.values_of_lossy("FILES").unwrap_or_default(),
                    verbose : cmd.is_present("verbose") || verbose
                }
            },
            ("stage", Some(cmd)) => {
                Command::Stage {
                    files   : cmd.values_of_lossy("FILES").unwrap_or_default(),
                    verbose : cmd.is_present("verbose") || verbose,
                    discard_workdir_changes : cmd.is_present("discard-external-changes")
                }
            },            
            ("reset", Some(cmd)) => {
                Command::Reset {
                    files   : cmd.values_of_lossy("FILES").unwrap_or_default(),
                    verbose : cmd.is_present("verbose") || verbose,
                    force   : cmd.is_present("force")
                }
            },                        
            ("gitfilter", Some(cmd)) if cmd.is_present("clean") && !cmd.is_present("smudge") => {
                Command::FilterClean {
                    path: cmd.value_of_lossy("clean").expect("missing PATH").into()
                }
            },
            ("gitfilter", Some(cmd)) if cmd.is_present("smudge") && !cmd.is_present("clean") => {
                Command::FilterSmudge {
                    path: cmd.value_of_lossy("smudge").expect("missing PATH").into()
                }
            },
            ("show", Some(cmd)) => {
                Command::Reconstruct {
                    pathspec : cmd.value_of_lossy("PATHSPEC").expect("missing PATHSPEC").into(),
                    bare     : cmd.is_present("bare")
                }
            },            
            // otherwise
            _ => {
                panic!("unknown command line command");
            }
        };

        Ok( command )
    }
}




