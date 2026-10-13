//! Translation of `c_src/tests/run-loader.c`.

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
use crate::api_document::*;
use crate::ctest::*;
use crate::loader::*;

unsafe fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        let mut number: c_int;

        if argc < 2 {
            printf(cstr!("Usage: %s file1.yaml ...\n"), *argv.offset(0));
            return 0;
        }

        number = 1;
        while number < argc {
            let file: *mut FILE;
            let mut parser: yaml_parser_t = core::mem::zeroed();
            let mut document: yaml_document_t = core::mem::zeroed();
            let mut done: c_int = 0;
            let mut count: c_int = 0;
            let mut error: c_int = 0;

            printf(
                cstr!("[%d] Loading '%s': "),
                number,
                *argv.offset(number as isize),
            );
            fflush(stdout);

            file = fopen(*argv.offset(number as isize), cstr!("rb"));
            c_assert!(file);

            c_assert!(yaml_parser_initialize(&mut parser));

            yaml_parser_set_input_file(&mut parser, file);

            while done == 0 {
                if yaml_parser_load(&mut parser, &mut document) == 0 {
                    error = 1;
                    break;
                }

                done = (yaml_document_get_root_node(&mut document).is_null()) as c_int;

                yaml_document_delete(&mut document);

                if done == 0 {
                    count += 1;
                }
            }

            yaml_parser_delete(&mut parser);

            c_assert!(fclose(file) == 0);

            printf(
                cstr!("%s (%d documents)\n"),
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
