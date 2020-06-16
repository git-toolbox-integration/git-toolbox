//
// src/listing_formatter.rs 
//
// A bare-bones implementation for prettified display of file listings with issues.  
//
// (C) 2020 Taras Zakharko
//
// This code is licensed under GPL 3.0

use std::fmt;
use tap::*;


#[derive(Debug)]
pub struct ListingLine {
    line_number : usize, 
    line_text   : String,
    marker      : Option<(usize, String)>,
    notes       : Vec<String>
}

/// Writes out a nicely formatted listing
#[derive(Debug)]
pub struct ListingFormatter {
    label         : Option<String>,
    lines         : Vec<ListingLine>,
    surround      : usize
}

impl ListingLine {
  pub fn set_marker<S: Into<String>>(&mut self, offset : usize, text : S) {
    use console::measure_text_width;

    // set the new value
    self.marker.replace({
        // check that the marker is within the bounds of the line
        assert!(
            offset <= measure_text_width(&self.line_text), 
            "marker offset must be within the line width"
        );

        (offset, text.into())
    })
    .tap_some(|_| {
        panic!("Marker is already set");
    });
  } 
}

impl ListingFormatter {
    pub fn new_with_issue<S, M>(
        text: S, at_line: usize, offset: usize, message : M
    ) -> Self
    where 
        S: AsRef<str>,
        M: Into<String> 
    {
        let mut listing = ListingFormatter::new();

        let lines = text.as_ref().lines().enumerate().filter(|&(i, _)| {
            ((i+1) >= at_line.saturating_sub(2)) &&
            ((i+1) <= at_line.checked_add(2).unwrap_or(i))
        });

        for (i, text) in lines {
            listing.push_line(i+1, text);
        }

        listing.lines.iter_mut().find(|line| line.line_number == at_line).tap_some(|line| {
            line.set_marker(offset, message);
        }); 

        listing
    }

    pub fn new() -> ListingFormatter {
        ListingFormatter {
            label : None, 
            lines : vec!(),
            surround : 0
        }
    }

    pub fn set_label<S: Into<String>>(&mut self, label : S) -> &mut Self {
        assert!(self.label.is_none(), "self.label already set to {}!", self.label.as_ref().unwrap());
        self.label.replace(label.into());
  
        self
    }

    pub fn push_line<S>(&mut self, line_number : usize, line_text : S) -> &mut ListingLine 
    where
        S: Into<String>
    {
        self.lines.last().tap_some(|last| {
            assert!(
                last.line_number < line_number, 
                "attempt to add line {} after line {}", 
                line_number, 
                last.line_number);
        });

        let line_text = line_text.into();
        let line_text = if line_text.is_empty() {
            " ".to_owned()
        } else {
            line_text
        };
  
        self.lines.push(ListingLine {
            line_number,
            line_text,
            marker    : None,
            notes: vec!()
        });
  
        self.lines.last_mut().unwrap()
    }
}


impl fmt::Display for ListingFormatter {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use console::{measure_text_width, truncate_str}; 
    
        use textwrap::wrap_iter;
    
        // the wrap border
        let wrap_at = formatter.width().unwrap_or(80);
    
        // early bail out if nothign to print
        if self.lines.is_empty() {
            writeln!(formatter)?;
            writeln!(formatter, "{}", truncate_str("  ...empty listing...  ", wrap_at, "..."))?;
            writeln!(formatter)?;
            return Ok( () );
        }
    
        // compute the width of the line number field margin
        let line_number_width = num_digits(
          self.lines.iter().fold(0, |max, line| std::cmp::max(max, line.line_number))
        );
    

        // setup the label
        let mut label_text = self.label.as_ref()
            .map(|label| format!("{}: ", label))
            .unwrap_or_default();

        // the label width
        let label_width = measure_text_width(&label_text);

        // the margin width
        let margin_area_width = line_number_width + label_width;
    
        // compute the width of the text area
        // 5 is the width of the additional padding and the divider
        let text_area_width = wrap_at.saturating_sub(margin_area_width + 5);
    
        // compute the width of the marker text area
        let marker_text_width = ((text_area_width as f64)*0.8).trunc() as usize;

        // if the text area is too short...
        if marker_text_width == 0 {
          return Ok( () );
        };


        // do the basic writing
        for line in self.lines.iter() {
            // split the line text into wrapped lines
            let wrapped = wrap_iter(&line.line_text, text_area_width);
    
            // whether it is the first line to draw
            let mut is_first = true;
            // the total outputted string width
            let mut rendered_width = 0;
    
            for wrapped_line in wrapped {
                // draw the line
                if is_first {
                    writeln!(formatter, "  {:>label_width$}{:>line_number_width$} | {}",
                        std::mem::take(&mut label_text),
                        line.line_number,
                        wrapped_line,
                        label_width = label_width,
                        line_number_width = line_number_width
                    )?;

                    is_first = false;
                } else {
                    writeln!(formatter, "  {:>margin_area_width$} | {}",
                        "", // placeholder
                        wrapped_line,
                        margin_area_width = margin_area_width
                    )?;
                };

                // get the line rendered width
                let width = measure_text_width(&wrapped_line);
    
                // draw the marker if nessesary
                let draw_marker = line.marker.as_ref().map(|&(offset, _)| {
                    offset > rendered_width && offset <= rendered_width + width
                }).unwrap_or(false);

                if draw_marker {
                    // get the marker data
                    let (offset, marker) = line.marker.as_ref().unwrap();
    
                    // adjust the offset
                    let offset = offset.checked_sub(rendered_width + 1).unwrap_or(0);
    
                    // display the marker itself
                    writeln!(formatter, "  {:>margin_area_width$} | {:>offset$}^", 
                        "", // placeholder for number marker
                        "", // placeholder for the offset
                        margin_area_width = margin_area_width,
                        offset = offset
                    )?;
    
                    if !&marker.trim().is_empty() {
                        writeln!(formatter, "  {:>margin_area_width$} |", 
                            "", // placeholder for the margin, 
                            margin_area_width = margin_area_width
                        )?;
                        for wrapped_line in wrap_iter(&marker, marker_text_width) {
                            writeln!(formatter, "  {:>margin_area_width$} |   {}", 
                                "", // placeholder for number marker
                                &wrapped_line,
                                margin_area_width = margin_area_width
                            )?;
                        };
                        writeln!(formatter, "  {:>margin_area_width$} |", 
                            "", // placeholder for the margin, 
                            margin_area_width = margin_area_width
                        )?;
                    }
                }

                // increase the rendered width
                rendered_width += width;
            }
        }

        Ok( () )
    }
}

// I felt "inspired"
fn num_digits(x: usize) -> usize {
    match x {
        0                    ..= 9 => 1,
        10                   ..= 99 => 2,
        100                  ..= 999 => 3,
        1000                 ..= 9999 => 4,
        10000                ..= 99999 => 5,
        100000               ..= 999999 => 6,
        1000000              ..= 9999999 => 7,
        10000000             ..= 99999999 => 8,
        100000000            ..= 999999999 => 9,
        1000000000           ..= 9999999999 => 10,
        10000000000          ..= 99999999999 => 11,
        100000000000         ..= 999999999999 => 12,
        1000000000000        ..= 9999999999999 => 13,
        10000000000000       ..= 99999999999999 => 14,
        100000000000000      ..= 999999999999999 => 15,
        1000000000000000     ..= 9999999999999999 => 16,
        10000000000000000    ..= 99999999999999999 => 17,
        100000000000000000   ..= 999999999999999999 => 18,
        _ => panic!("This number is way too high...")
    }
}