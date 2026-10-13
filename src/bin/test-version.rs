//! Translation of `c_src/tests/test-version.c`.

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

use crate::ctest::*;
use crate::memory::*;

unsafe fn c_main() -> c_int {
    unsafe {
        let mut major: c_int = -1;
        let mut minor: c_int = -1;
        let mut patch: c_int = -1;
        let mut buf: [c_char; 64] = [0; 64];

        yaml_get_version(&mut major, &mut minor, &mut patch);
        sprintf(buf.as_mut_ptr(), cstr!("%d.%d.%d"), major, minor, patch);
        c_assert!(strcmp(buf.as_ptr(), yaml_get_version_string()) == 0);

        /* Print structure sizes. */
        printf(
            cstr!("sizeof(token) = %ld\n"),
            core::mem::size_of::<yaml_token_t>() as c_long,
        );
        printf(
            cstr!("sizeof(event) = %ld\n"),
            core::mem::size_of::<yaml_event_t>() as c_long,
        );
        printf(
            cstr!("sizeof(parser) = %ld\n"),
            core::mem::size_of::<yaml_parser_t>() as c_long,
        );

        0
    }
}

fn main() {
    ctest::run_void(c_main)
}
