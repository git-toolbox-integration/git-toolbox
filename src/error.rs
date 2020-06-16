//
// src/error.rs 
//
// The global error catalogue. All errors are defined here. 
// In addition, common error styling routines are here as well. 
// 
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use std::path::PathBuf;
use crate::util::get_relative_path;

define_error!(
    InvalidRepository 
    @display(self) {
        (@err "unable to locate the git repository")
        (@div "Are you running {cmd} from outside your git project?" 
            [
                cmd = style::command("git toolbox")
            ])
    }
);

define_error!(
    InvalidManagedPath {
        pub path: String
    }
    @display(self) {
        (@err "invalid characters in a managed path {path}" 
            [
                path = style::path(&self.path)
            ]
        )
    }
);

define_error!(
    PathNotInRepository {
        pub path: PathBuf
    }
    @display(self) {
        (@err "{path} is not within the repository" 
            [
                path = style::path(&self.path.display())
            ]
        )
    }
);


define_error!(
    UnableToStageManagedFile {
        pub path: PathBuf
    }
    @display(self) {
        (@err "{path} is a managed file and cannot be staged manually"
              "(use {cmd} to stage it)" 
            [
                path = style::path(&self.path.display()),
                cmd = style::command(format!("git toolbox stage {}", &self.path.display()))
            ]
        )
    }
);


define_error!(
    ExternalModificationsWillBeLost {
        pub path: PathBuf
    }
    @display(self) {
        (@err "some external modifications to the managed path {path} would be lost"
            [
                path = style::path(&self.path.display()),
            ]
        )
    }
);

define_error!(
    NotAManagedFile {
        pub path: PathBuf
    }
    @display(self) {
        (@err "{path} does not exist or is not a managed file"
            [
                path = style::path(&self.path.display())
            ]
        )
    }
);



define_error!(
    InvalidClobPath {
        pub path: String
    }
    @display(self) {
        (@err "invalid characters in git artefact name {path}" 
              "This artifact will be ignored."
            [
                path = style::path(&self.path)
            ]
        )
    }
);


define_error!(
    GitObjNotFound {
        pub path: String,
        pub rev : String
    }
    @display(self) {
        (@err "{path} not found in {rev}" 
            [
                path = style::path(&self.path),
                rev  = &self.rev,
            ]
        )
    }
);

define_error!(
    GitRevisionNotFound {
        pub rev : String
    }
    @display(self) {
        (@err "invalid git revision {rev}" 
            [
                rev  = &self.rev,
            ]
        )
    }
);

define_error!(
    InvalidPathSpec {
        pub pathspec: String
    }
    @display(self) {
        (@err "{pathspec} is not valid git path specification"
            [
                pathspec = style::value(&self.pathspec)
            ]
        )
    }
);

define_error!(
    OtherGitError {
        pub msg : String
    }
    @display(self) {
        (@err "git error {msg}" [
                msg  = style::comment(&self.msg)
            ]
        )
    }
);

impl From<git2::Error> for OtherGitError {
    fn from(error: git2::Error) -> Self {
        OtherGitError { msg : error.message().to_owned() }
    }
}

define_error!(
    FileWriteError {
        pub path : PathBuf,
        pub msg  : String,
    }
    @display(self) {
        (@err "unable to write {path} {msg}" 
            [
                path = style::path(get_relative_path(&self.path).display()),
                msg  = style::comment(&self.msg)
            ]
        )
    }
);

define_error!(
    FileReadError {
        pub path : PathBuf,
        pub msg  : String,
    }
    @display(self) {
        (@err "unable to read {path} {msg}" 
            [
                path = style::path(get_relative_path(&self.path).display()),
                msg  = style::comment(&self.msg)
            ]
        )
    }
);

define_error!(
    FileDeleteError {
        pub path : PathBuf,
        pub msg  : String,
    }
    @display(self) {
        (@err "unable to delete {path} {msg}" 
            [
                path = style::path(get_relative_path(&self.path).display()),
                msg  = style::comment(&self.msg)
            ]
        )
    }
);

define_error!(
    FileNotFound {
        pub path : PathBuf,
    }
    @display(self) {
        (@err "{path} not found" 
            [
                path = style::path(get_relative_path(&self.path).display())
            ]
        )
    }
);

define_error!(
    ToolboxDictionaryMissingHeader {
        pub path : PathBuf,
        pub text : &'static str,
        pub line : usize
    }
    @display(self) {
        (@err "toolbox dictinary header missing or invalid in {path}"
            [
                path = style::path(get_relative_path(&self.path).display())
            ] 
        )
        (@div "{body}" 
            [
                body={
                    use crate::listing_formatter::ListingFormatter;

                    let style = console::Style::new().italic().yellow();
                    let path  = get_relative_path(&self.path);

                    // setup the listing
                    let mut listing = ListingFormatter::new_with_issue(
                        self.text, self.line+1, 0, "expected '\\_sh v3.0  ...  Dictionary' here"
                    );
                    listing.set_label(style.apply_to(path.display()).to_string());

                    // write the error message
                    format!("{:80}", listing)
                }
            ]
        )
    }       
);

define_error!(
    ConfigurationChanged
    @display(self) {
        (@err "configuration file {path} has changed" 
            [
                path=crate::config::CONFIG_FILE
            ]
        )
        (@div "Please run {cmd} before proceeding" 
            [
                cmd=style::command("git toolbox setup")
            ]
        )
    }       
);

define_error!(
    ConfigurationNeeded
    @display(self) {
        (@err "the repository needs to be configured")
        (@div "Please run {cmd} before proceeding" 
            [
                cmd=style::command("git toolbox setup")
            ]
        )
    }       
);

define_error!(
    ConfigurationMissing
    @display(self) {
        (@err "configuration file {path} has is missing" 
            [
                path=crate::config::CONFIG_FILE
            ]
        )
        (@div "Please provide a valid configuration and run {cmd} before proceeding"
            [
                cmd=style::command("git toolbox setup")
            ]
        )
        (@div "The command {cmd} will generade an example configuration file for you"
            [
                cmd=style::command("git toolbox setup --init")
            ]
        )

    }       
);

define_error!(
    ConfigurationExists
    @display(self) {
        (@err "configuration file {path} already exists" 
            [
                path=crate::config::CONFIG_FILE
            ]
        )
    }       
);

define_error!(
    ConfigurationError {
        pub text : String,
        pub at   : Option<(usize, usize)>,
        pub msg  : String
    }
    @display(self) {
        (@err "malformated configuration" 
        )
        (@div "{body}" 
            [
                body={
                    let style = console::Style::new().italic();

                    if let Some( (row, col) ) = self.at {
                        use crate::listing_formatter::ListingFormatter;
                        // setup the listing
                        let mut listing = ListingFormatter::new_with_issue(
                            &self.text, row+1, col+1, &self.msg
                        );
                        listing.set_label(style.apply_to(crate::config::CONFIG_FILE).to_string());
                        // write the error message
                        format!("{:80}", listing)
                    } else {
                        // no location information, just write a short message
                        format!("{}: {}", style.apply_to(crate::config::CONFIG_FILE), self.msg)
                    }
                }
            ]
        )
    }       
);


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


mod style {
    use std::fmt::Display;
    use console::Style;

    fn basic_style() -> Style {
        Style::new().force_styling(true)
    }

    pub fn value<D: Display>(obj: D) -> impl Display {
        basic_style().cyan().apply_to(obj)
    }

    pub fn _prefix<D: Display>(obj: D) -> impl Display {
        basic_style().yellow().apply_to(obj)
    }

    pub fn comment<D: Display>(obj: D) -> impl Display {
        let msg = obj.to_string();
        let msg = if msg.trim().is_empty() {
            "".to_owned()
        } else {
            format!("({})", lowercase_first(&msg))   
        };

        basic_style().italic().apply_to(msg)
    }

    pub fn path<D: Display>(obj: D) -> impl Display {
        use crate::util::escape_unicode_only;

        basic_style().italic().apply_to(format!("'{}'", escape_unicode_only(&obj.to_string())))
    }

    pub fn command<D: Display>(obj: D) -> impl Display {
        basic_style().bold().apply_to(format!("`{}`", obj))
    }

    fn lowercase_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(ch) => ch.to_uppercase().chain(chars).collect(),
        }
    }
}