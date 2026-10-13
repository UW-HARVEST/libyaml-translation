//! Translation of `c_src/tests/test-nesting.c`.

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
use crate::scanner::*;

/*
 * Test that the scanner enforces the maximum nesting depth.
 */

unsafe fn scan_string(input: *const c_char, length: size_t) -> c_int {
    unsafe {
        let mut parser: yaml_parser_t = core::mem::zeroed();
        let mut token: yaml_token_t = core::mem::zeroed();
        let mut done: c_int = 0;
        let mut error: c_int = 0;

        c_assert!(yaml_parser_initialize(&mut parser));
        yaml_parser_set_input_string(&mut parser, input as *const c_uchar, length);

        while done == 0 {
            if yaml_parser_scan(&mut parser, &mut token) == 0 {
                error = 1;
                break;
            }
            done = (token.type_ == YAML_STREAM_END_TOKEN) as c_int;
            yaml_token_delete(&mut token);
        }

        yaml_parser_delete(&mut parser);
        error
    }
}

unsafe fn c_main() -> c_int {
    unsafe {
        let mut input: *mut c_char;
        let mut i: c_int;

        /* Test 1: nesting beyond the default limit (1000) must fail. */
        {
            let depth: c_int = 2000;
            input = malloc((depth + 1) as size_t) as *mut c_char;
            c_assert!(input);
            memset(input as *mut c_void, '[' as c_int, depth as size_t);
            *input.offset(depth as isize) = '\0' as c_char;

            printf(cstr!("Test 1: %d nested '[' (exceeds limit) ... "), depth);
            fflush(stdout);
            c_assert!(scan_string(input, depth as size_t) == 1);
            printf(cstr!("OK (rejected)\n"));
            free(input as *mut c_void);
        }

        /* Test 2: block nesting beyond the limit must fail. */
        {
            /*
             * Build a YAML string with 2000 levels of block mapping nesting:
             * "a:\n b:\n  c:\n   d:\n..." - each level indented one more space.
             */
            let depth: c_int = 2000;
            let mut len: c_int = 0;
            let mut pos: c_int = 0;

            /* Each level: indent spaces + "X:\n" */
            i = 0;
            while i < depth {
                len += i + 2 + 1; /* i spaces + "X:" + newline */
                i += 1;
            }
            input = malloc((len + 1) as size_t) as *mut c_char;
            c_assert!(input);

            i = 0;
            while i < depth {
                let mut j: c_int;
                j = 0;
                while j < i {
                    *input.offset(pos as isize) = ' ' as c_char;
                    pos += 1;
                    j += 1;
                }
                *input.offset(pos as isize) = (('a' as c_int) + (i % 26)) as c_char;
                pos += 1;
                *input.offset(pos as isize) = ':' as c_char;
                pos += 1;
                *input.offset(pos as isize) = '\n' as c_char;
                pos += 1;
                i += 1;
            }
            *input.offset(pos as isize) = '\0' as c_char;

            printf(
                cstr!("Test 2: %d levels block nesting (exceeds limit) ... "),
                depth,
            );
            fflush(stdout);
            c_assert!(scan_string(input, pos as size_t) == 1);
            printf(cstr!("OK (rejected)\n"));
            free(input as *mut c_void);
        }

        /* Test 3: flow nesting within the limit must succeed. */
        {
            let depth: c_int = 500;
            let len: c_int = depth * 2;
            input = malloc((len + 1) as size_t) as *mut c_char;
            c_assert!(input);
            i = 0;
            while i < depth {
                *input.offset(i as isize) = '[' as c_char;
                i += 1;
            }
            i = 0;
            while i < depth {
                *input.offset((depth + i) as isize) = ']' as c_char;
                i += 1;
            }
            *input.offset(len as isize) = '\0' as c_char;

            printf(cstr!("Test 2: %d nested '[]' (within limit) ... "), depth);
            fflush(stdout);
            c_assert!(scan_string(input, len as size_t) == 0);
            printf(cstr!("OK (accepted)\n"));
            free(input as *mut c_void);
        }

        0
    }
}

fn main() {
    ctest::run_void(c_main)
}
