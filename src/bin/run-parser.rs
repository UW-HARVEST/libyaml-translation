//! Translation of `c_src/tests/run-parser.c`.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_unsafe)]

#[path = "../lib.rs"]
mod yamllib;
use yamllib::*;

#[macro_use]
#[path = "../ctest.rs"]
mod ctest;

use crate::api::*;
use crate::api_objects::*;
use crate::ctest::*;
use crate::parser::*;

unsafe fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        let mut number: c_int;
        let mut start: c_int = 0;
        let mut i: c_int = 0;
        let mut filename: *mut c_char;
        let mut output: *mut c_char = core::ptr::null_mut();
        let mut max_level: c_int;
        let mut show_error: c_int = 0;

        if argc < 2 {
            printf(cstr!("Usage: %s file1.yaml ...\n"), *argv.offset(0));
            return 0;
        }
        i = 1;
        while i < argc {
            if strncmp(*argv.offset(i as isize), cstr!("--max-level"), 11) == 0 {
                i += 1;
                max_level = strtol(*argv.offset(i as isize), &mut output, 10) as c_int;
                yaml_set_max_nest_level(max_level);
                start = i + 1;
            } else if strncmp(*argv.offset(i as isize), cstr!("--show-error"), 12) == 0 {
                show_error = 1;
                start = i + 1;
            }
            i += 1;
        }

        number = start;
        while number < argc {
            let file: *mut FILE;
            let mut parser: yaml_parser_t = core::mem::zeroed();
            let mut event: yaml_event_t = core::mem::zeroed();
            let mut done: c_int = 0;
            let mut count: c_int = 0;
            let mut error: c_int = 0;

            filename = *argv.offset(number as isize);
            printf(cstr!("[%d] Parsing '%s': "), number, filename);
            fflush(stdout);

            file = fopen(filename, cstr!("rb"));
            c_assert!(file);

            c_assert!(yaml_parser_initialize(&mut parser));

            yaml_parser_set_input_file(&mut parser, file);

            while done == 0 {
                if yaml_parser_parse(&mut parser, &mut event) == 0 {
                    error = 1;
                    if show_error != 0 {
                        fprintf(
                            stderr,
                            cstr!("Parse error: %s\nLine: %lu Column: %lu\n"),
                            parser.problem,
                            (parser.problem_mark.line as u64) + 1,
                            (parser.problem_mark.column as u64) + 1,
                        );
                    }
                    break;
                }

                done = (event.type_ == YAML_STREAM_END_EVENT) as c_int;

                yaml_event_delete(&mut event);

                count += 1;
            }

            yaml_parser_delete(&mut parser);

            c_assert!(fclose(file) == 0);

            printf(
                cstr!("%s (%d events)\n"),
                if error != 0 {
                    cstr!("FAILURE")
                } else {
                    cstr!("SUCCESS")
                },
                count,
            );

            number += 1;
        }

        0
    }
}

fn main() {
    ctest::run(c_main)
}
