//
// src/error_macros.rs 
//
// Convenince macros for declaring error types (see src/error.rs for usage examples) 
//
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

macro_rules! fmt_err_msg {
    // error header
    ((@err $($msg:literal)+)) => {
        format!("{} {}", 
            // styled error: marker
            ::console::Style::new().red().bold().apply_to("error:"), 
            // the header itself
            format!(concat!($($msg, " "),+))
        )    
    };
    ((@err $($msg:literal)+ [ $($arg:tt)* ])) => {
        format!("{} {}", 
            // styled error: marker
            ::console::Style::new().red().bold().apply_to("error:"), 
            // the header itself
            format!(concat!($($msg, " "),+), $($arg)*)
        )    
    };
    // error body
    ((@div $($msg:literal)+ )) => {
        format!(concat!($($msg, " "),+))    
    };
    ((@div $($msg:literal)+ [ $($arg:tt)* ])) => {
        format!(concat!($($msg, " "),+), $($arg)*)    
    };    
    // a separator
    ((@sep )) => {
        "------".to_owned()
    };
}


macro_rules! define_error {
    ($name:ident  @display($sel:ident) { $($msg:tt)* }) => {
        #[derive(Debug)]
        pub struct $name;

        impl std::error::Error for $name {} 

        impl std::fmt::Display for $name {
            fn fmt(&$sel, __formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                let msg = [$((fmt_err_msg!($msg)),)+].join("\n\n");
                
                __formatter.write_str(&msg)
            }
        }    
    };
    ($name:ident { $($elem:tt)* } @display($sel:ident) { $($msg:tt)* }) => {
        #[derive(Debug)]
        pub struct $name {
            $($elem)*
        }

        impl std::error::Error for $name {} 

        impl std::fmt::Display for $name {
            fn fmt(&$sel, __formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                let msg = [$((fmt_err_msg!($msg)),)+].join("\n\n");
                
                __formatter.write_str(&msg)
            }
        }    
    };
}