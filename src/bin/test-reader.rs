//! Translation of `c_src/tests/test-reader.c`.

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
use crate::ctest::*;
use crate::reader::*;

/*
 * Test cases are stolen from
 * http://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-test.txt
 */

/// `typedef struct { char *title; char *test; int result; } test_case;`
///
/// The C table stores plain `char *`; raw pointers are not `Sync`, so the Rust
/// transcription keeps the same bytes as NUL-terminated `&'static [u8]` slices
/// and takes `.as_ptr()` at the use sites.  The `{NULL, NULL, 0}` sentinel
/// becomes a pair of empty slices and the loop condition `table[k].test`
/// becomes `!table[k].test.is_empty()`.
struct test_case {
    title: &'static [u8],
    test: &'static [u8],
    result: c_int,
}

static utf8_sequences: [test_case; 43] = [
    /* {"title", "test 1|test 2|...|test N!", (0 or 1)}, */

    test_case { title: b"a simple test\0", test: b"'test' is '\xd0\xbf\xd1\x80\xd0\xbe\xd0\xb2\xd0\xb5\xd1\x80\xd0\xba\xd0\xb0' in Russian!\0", result: 1 },
    test_case { title: b"an empty line\0", test: b"!\0", result: 1 },

    test_case { title: b"u-0 is a control character\0", test: b"\x00!\0", result: 0 },
    test_case { title: b"u-80 is a control character\0", test: b"\xc2\x80!\0", result: 0 },
    test_case { title: b"u-800 is valid\0", test: b"\xe0\xa0\x80!\0", result: 1 },
    test_case { title: b"u-10000 is valid\0", test: b"\xf0\x90\x80\x80!\0", result: 1 },
    test_case { title: b"5 bytes sequences are not allowed\0", test: b"\xf8\x88\x80\x80\x80!\0", result: 0 },
    test_case { title: b"6 bytes sequences are not allowed\0", test: b"\xfc\x84\x80\x80\x80\x80!\0", result: 0 },

    test_case { title: b"u-7f is a control character\0", test: b"\x7f!\0", result: 0 },
    test_case { title: b"u-7FF is valid\0", test: b"\xdf\xbf!\0", result: 1 },
    test_case { title: b"u-FFFF is a control character\0", test: b"\xef\xbf\xbf!\0", result: 0 },
    test_case { title: b"u-1FFFFF is too large\0", test: b"\xf7\xbf\xbf\xbf!\0", result: 0 },
    test_case { title: b"u-3FFFFFF is 5 bytes\0", test: b"\xfb\xbf\xbf\xbf\xbf!\0", result: 0 },
    test_case { title: b"u-7FFFFFFF is 6 bytes\0", test: b"\xfd\xbf\xbf\xbf\xbf\xbf!\0", result: 0 },

    test_case { title: b"u-D7FF\0", test: b"\xed\x9f\xbf!\0", result: 1 },
    test_case { title: b"u-E000\0", test: b"\xee\x80\x80!\0", result: 1 },
    test_case { title: b"u-FFFD\0", test: b"\xef\xbf\xbd!\0", result: 1 },
    test_case { title: b"u-10FFFF\0", test: b"\xf4\x8f\xbf\xbf!\0", result: 1 },
    test_case { title: b"u-110000\0", test: b"\xf4\x90\x80\x80!\0", result: 0 },

    test_case { title: b"first continuation byte\0", test: b"\x80!\0", result: 0 },
    test_case { title: b"last continuation byte\0", test: b"\xbf!\0", result: 0 },

    test_case { title: b"2 continuation bytes\0", test: b"\x80\xbf!\0", result: 0 },
    test_case { title: b"3 continuation bytes\0", test: b"\x80\xbf\x80!\0", result: 0 },
    test_case { title: b"4 continuation bytes\0", test: b"\x80\xbf\x80\xbf!\0", result: 0 },
    test_case { title: b"5 continuation bytes\0", test: b"\x80\xbf\x80\xbf\x80!\0", result: 0 },
    test_case { title: b"6 continuation bytes\0", test: b"\x80\xbf\x80\xbf\x80\xbf!\0", result: 0 },
    test_case { title: b"7 continuation bytes\0", test: b"\x80\xbf\x80\xbf\x80\xbf\x80!\0", result: 0 },

    test_case { title: b"sequence of all 64 possible continuation bytes\0",
     test: b"\x80|\x81|\x82|\x83|\x84|\x85|\x86|\x87|\x88|\x89|\x8a|\x8b|\x8c|\x8d|\x8e|\x8f|\x90|\x91|\x92|\x93|\x94|\x95|\x96|\x97|\x98|\x99|\x9a|\x9b|\x9c|\x9d|\x9e|\x9f|\xa0|\xa1|\xa2|\xa3|\xa4|\xa5|\xa6|\xa7|\xa8|\xa9|\xaa|\xab|\xac|\xad|\xae|\xaf|\xb0|\xb1|\xb2|\xb3|\xb4|\xb5|\xb6|\xb7|\xb8|\xb9|\xba|\xbb|\xbc|\xbd|\xbe|\xbf!\0", result: 0 },
    test_case { title: b"32 first bytes of 2-byte sequences {0xc0-0xdf}\0",
     test: b"\xc0 |\xc1 |\xc2 |\xc3 |\xc4 |\xc5 |\xc6 |\xc7 |\xc8 |\xc9 |\xca |\xcb |\xcc |\xcd |\xce |\xcf |\xd0 |\xd1 |\xd2 |\xd3 |\xd4 |\xd5 |\xd6 |\xd7 |\xd8 |\xd9 |\xda |\xdb |\xdc |\xdd |\xde |\xdf !\0", result: 0 },
    test_case { title: b"16 first bytes of 3-byte sequences {0xe0-0xef}\0",
     test: b"\xe0 |\xe1 |\xe2 |\xe3 |\xe4 |\xe5 |\xe6 |\xe7 |\xe8 |\xe9 |\xea |\xeb |\xec |\xed |\xee |\xef !\0", result: 0 },
    test_case { title: b"8 first bytes of 4-byte sequences {0xf0-0xf7}\0", test: b"\xf0 |\xf1 |\xf2 |\xf3 |\xf4 |\xf5 |\xf6 |\xf7 !\0", result: 0 },
    test_case { title: b"4 first bytes of 5-byte sequences {0xf8-0xfb}\0", test: b"\xf8 |\xf9 |\xfa |\xfb !\0", result: 0 },
    test_case { title: b"2 first bytes of 6-byte sequences {0xfc-0xfd}\0", test: b"\xfc |\xfd !\0", result: 0 },

    test_case { title: b"sequences with last byte missing {u-0}\0",
     test: b"\xc0|\xe0\x80|\xf0\x80\x80|\xf8\x80\x80\x80|\xfc\x80\x80\x80\x80!\0", result: 0 },
    test_case { title: b"sequences with last byte missing {u-...FF}\0",
     test: b"\xdf|\xef\xbf|\xf7\xbf\xbf|\xfb\xbf\xbf\xbf|\xfd\xbf\xbf\xbf\xbf!\0", result: 0 },

    test_case { title: b"impossible bytes\0", test: b"\xfe|\xff|\xfe\xfe\xff\xff!\0", result: 0 },

    test_case { title: b"overlong sequences {u-2f}\0",
     test: b"\xc0\xaf|\xe0\x80\xaf|\xf0\x80\x80\xaf|\xf8\x80\x80\x80\xaf|\xfc\x80\x80\x80\x80\xaf!\0", result: 0 },

    test_case { title: b"maximum overlong sequences\0",
     test: b"\xc1\xbf|\xe0\x9f\xbf|\xf0\x8f\xbf\xbf|\xf8\x87\xbf\xbf\xbf|\xfc\x83\xbf\xbf\xbf\xbf!\0", result: 0 },

    test_case { title: b"overlong representation of the NUL character\0",
     test: b"\xc0\x80|\xe0\x80\x80|\xf0\x80\x80\x80|\xf8\x80\x80\x80\x80|\xfc\x80\x80\x80\x80\x80!\0", result: 0 },

    test_case { title: b"single UTF-16 surrogates\0",
     test: b"\xed\xa0\x80|\xed\xad\xbf|\xed\xae\x80|\xed\xaf\xbf|\xed\xb0\x80|\xed\xbe\x80|\xed\xbf\xbf!\0", result: 0 },

    test_case { title: b"paired UTF-16 surrogates\0",
     test: b"\xed\xa0\x80\xed\xb0\x80|\xed\xa0\x80\xed\xbf\xbf|\xed\xad\xbf\xed\xb0\x80|\xed\xad\xbf\xed\xbf\xbf|\xed\xae\x80\xed\xb0\x80|\xed\xae\x80\xed\xbf\xbf|\xed\xaf\xbf\xed\xb0\x80|\xed\xaf\xbf\xed\xbf\xbf!\0", result: 0 },

    test_case { title: b"other illegal code positions\0", test: b"\xef\xbf\xbe|\xef\xbf\xbf!\0", result: 0 },

    test_case { title: b"", test: b"", result: 0 },
];

static boms: [test_case; 5] = [

    /* {"title", "test!", length}, */

    test_case { title: b"no bom (utf-8)\0", test: b"Hi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82!\0", result: 13 },
    test_case { title: b"bom (utf-8)\0", test: b"\xef\xbb\xbfHi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82!\0", result: 13 },
    test_case { title: b"bom (utf-16-le)\0", test: b"\xff\xfeH\x00i\x00 \x00i\x00s\x00 \x00\x1f\x04@\x048\x042\x045\x04B\x04!\0", result: 13 },
    test_case { title: b"bom (utf-16-be)\0", test: b"\xfe\xff\x00H\x00i\x00 \x00i\x00s\x00 \x04\x1f\x04@\x048\x042\x045\x04B!\0", result: 13 },
    test_case { title: b"", test: b"", result: 0 },
];

static bom_original: &[u8] = b"Hi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82\0";

unsafe fn check_utf8_sequences() -> c_int {
    unsafe {
        let mut parser: yaml_parser_t = core::mem::zeroed();
        let mut failed: c_int = 0;
        let mut k: c_int;
        printf(cstr!("checking utf-8 sequences...\n"));
        k = 0;
        while !utf8_sequences[k as usize].test.is_empty() {
            let title: *const c_char = utf8_sequences[k as usize].title.as_ptr() as *const c_char;
            let check: c_int = utf8_sequences[k as usize].result;
            let mut result: c_int;
            let mut start: *const c_uchar = utf8_sequences[k as usize].test.as_ptr();
            let mut end: *const c_uchar = start;
            printf(cstr!("\t%s:\n"), title);
            loop {
                while *end != b'|' && *end != b'!' {
                    end = end.add(1);
                }
                yaml_parser_initialize(&mut parser);
                yaml_parser_set_input_string(
                    &mut parser,
                    start,
                    (end as usize - start as usize) as size_t,
                );
                result = yaml_parser_update_buffer(
                    &mut parser,
                    (end as usize - start as usize) as size_t,
                );
                if result != check {
                    printf(cstr!("\t\t- "));
                    failed += 1;
                } else {
                    printf(cstr!("\t\t+ "));
                }
                if parser.error == 0 {
                    printf(cstr!("(no error)\n"));
                } else if parser.error == YAML_READER_ERROR {
                    if parser.problem_value != -1 {
                        printf(
                            cstr!("(reader error: %s: #%X at %ld)\n"),
                            parser.problem,
                            parser.problem_value as c_uint,
                            parser.problem_offset as c_long,
                        );
                    } else {
                        printf(
                            cstr!("(reader error: %s at %ld)\n"),
                            parser.problem,
                            parser.problem_offset as c_long,
                        );
                    }
                }
                if *end == b'!' {
                    break;
                }
                end = end.add(1);
                start = end;
                yaml_parser_delete(&mut parser);
            }
            printf(cstr!("\n"));
            k += 1;
        }
        printf(cstr!("checking utf-8 sequences: %d fail(s)\n"), failed);
        failed
    }
}

unsafe fn check_boms() -> c_int {
    unsafe {
        let mut parser: yaml_parser_t = core::mem::zeroed();
        let mut failed: c_int = 0;
        let mut k: c_int;
        printf(cstr!("checking boms...\n"));
        k = 0;
        while !boms[k as usize].test.is_empty() {
            let title: *const c_char = boms[k as usize].title.as_ptr() as *const c_char;
            let check: c_int = boms[k as usize].result;
            let mut result: c_int;
            let start: *const c_uchar = boms[k as usize].test.as_ptr();
            let mut end: *const c_uchar = start;
            while *end != b'!' {
                end = end.add(1);
            }
            printf(cstr!("\t%s: "), title);
            yaml_parser_initialize(&mut parser);
            yaml_parser_set_input_string(
                &mut parser,
                start,
                (end as usize - start as usize) as size_t,
            );
            result =
                yaml_parser_update_buffer(&mut parser, (end as usize - start as usize) as size_t);
            if result == 0 {
                printf(
                    cstr!("- (reader error: %s at %ld)\n"),
                    parser.problem,
                    parser.problem_offset as c_long,
                );
                failed += 1;
            } else {
                if parser.unread != check as size_t {
                    printf(
                        cstr!("- (length=%ld while expected length=%d)\n"),
                        parser.unread as c_long,
                        check,
                    );
                    failed += 1;
                } else if memcmp(
                    parser.buffer.start as *const c_void,
                    bom_original.as_ptr() as *const c_void,
                    check as size_t,
                ) != 0
                {
                    printf(
                        cstr!("- (value '%s' does not equal to the original value '%s')\n"),
                        parser.buffer.start as *const c_char,
                        bom_original.as_ptr() as *const c_char,
                    );
                    failed += 1;
                } else {
                    printf(cstr!("+\n"));
                }
            }
            yaml_parser_delete(&mut parser);
            k += 1;
        }
        printf(cstr!("checking boms: %d fail(s)\n"), failed);
        failed
    }
}

const LONG: c_int = 100000;

unsafe fn check_long_utf8() -> c_int {
    unsafe {
        let mut parser: yaml_parser_t = core::mem::zeroed();
        let mut k: c_int = 0;
        let mut j: c_int;
        let mut failed: c_int = 0;
        let mut ch0: c_uchar;
        let mut ch1: c_uchar;
        let buffer: *mut c_uchar = malloc((3 + LONG * 2) as size_t) as *mut c_uchar;
        c_assert!(buffer);
        printf(cstr!("checking a long utf8 sequence...\n"));
        *buffer.offset(k as isize) = 0xef;
        k += 1;
        *buffer.offset(k as isize) = 0xbb;
        k += 1;
        *buffer.offset(k as isize) = 0xbf;
        k += 1;
        j = 0;
        while j < LONG {
            if (j % 2) != 0 {
                *buffer.offset(k as isize) = 0xd0;
                k += 1;
                *buffer.offset(k as isize) = 0x90;
                k += 1;
            } else {
                *buffer.offset(k as isize) = 0xd0;
                k += 1;
                *buffer.offset(k as isize) = 0xaf;
                k += 1;
            }
            j += 1;
        }
        yaml_parser_initialize(&mut parser);
        yaml_parser_set_input_string(&mut parser, buffer, (3 + LONG * 2) as size_t);
        k = 0;
        while k < LONG {
            if parser.unread == 0 {
                if yaml_parser_update_buffer(&mut parser, 1) == 0 {
                    printf(
                        cstr!("\treader error: %s at %ld\n"),
                        parser.problem,
                        parser.problem_offset as c_long,
                    );
                    failed = 1;
                    break;
                }
            }
            if parser.unread == 0 {
                printf(cstr!("\tnot enough characters at %d\n"), k);
                failed = 1;
                break;
            }
            if (k % 2) != 0 {
                ch0 = 0xd0;
                ch1 = 0x90;
            } else {
                ch0 = 0xd0;
                ch1 = 0xaf;
            }
            if *parser.buffer.pointer.offset(0) != ch0 || *parser.buffer.pointer.offset(1) != ch1 {
                printf(
                    cstr!("\tincorrect UTF-8 sequence: %X %X instead of %X %X\n"),
                    *parser.buffer.pointer.offset(0) as c_int as c_uint,
                    *parser.buffer.pointer.offset(1) as c_int as c_uint,
                    ch0 as c_int as c_uint,
                    ch1 as c_int as c_uint,
                );
                failed = 1;
                break;
            }
            parser.buffer.pointer = parser.buffer.pointer.offset(2);
            parser.unread -= 1;

            k += 1;
        }
        if failed == 0 {
            if yaml_parser_update_buffer(&mut parser, 1) == 0 {
                printf(
                    cstr!("\treader error: %s at %ld\n"),
                    parser.problem,
                    parser.problem_offset as c_long,
                );
                failed = 1;
            } else if *parser.buffer.pointer.offset(0) != b'\0' {
                printf(
                    cstr!("\texpected NUL, found %X (eof=%d, unread=%ld)\n"),
                    *parser.buffer.pointer.offset(0) as c_int as c_uint,
                    parser.eof,
                    parser.unread as c_long,
                );
                failed = 1;
            }
        }
        yaml_parser_delete(&mut parser);
        free(buffer as *mut c_void);
        printf(cstr!("checking a long utf8 sequence: %d fail(s)\n"), failed);
        failed
    }
}

unsafe fn check_long_utf16() -> c_int {
    unsafe {
        let mut parser: yaml_parser_t = core::mem::zeroed();
        let mut k: c_int = 0;
        let mut j: c_int;
        let mut failed: c_int = 0;
        let mut ch0: c_uchar;
        let mut ch1: c_uchar;
        let buffer: *mut c_uchar = malloc((2 + LONG * 2) as size_t) as *mut c_uchar;
        c_assert!(buffer);
        printf(cstr!("checking a long utf16 sequence...\n"));
        *buffer.offset(k as isize) = 0xff;
        k += 1;
        *buffer.offset(k as isize) = 0xfe;
        k += 1;
        j = 0;
        while j < LONG {
            if (j % 2) != 0 {
                *buffer.offset(k as isize) = 0x10;
                k += 1;
                *buffer.offset(k as isize) = 0x04;
                k += 1;
            } else {
                *buffer.offset(k as isize) = b'/';
                k += 1;
                *buffer.offset(k as isize) = 0x04;
                k += 1;
            }
            j += 1;
        }
        yaml_parser_initialize(&mut parser);
        yaml_parser_set_input_string(&mut parser, buffer, (2 + LONG * 2) as size_t);
        k = 0;
        while k < LONG {
            if parser.unread == 0 {
                if yaml_parser_update_buffer(&mut parser, 1) == 0 {
                    printf(
                        cstr!("\treader error: %s at %ld\n"),
                        parser.problem,
                        parser.problem_offset as c_long,
                    );
                    failed = 1;
                    break;
                }
            }
            if parser.unread == 0 {
                printf(cstr!("\tnot enough characters at %d\n"), k);
                failed = 1;
                break;
            }
            if (k % 2) != 0 {
                ch0 = 0xd0;
                ch1 = 0x90;
            } else {
                ch0 = 0xd0;
                ch1 = 0xaf;
            }
            if *parser.buffer.pointer.offset(0) != ch0 || *parser.buffer.pointer.offset(1) != ch1 {
                printf(
                    cstr!("\tincorrect UTF-8 sequence: %X %X instead of %X %X\n"),
                    *parser.buffer.pointer.offset(0) as c_int as c_uint,
                    *parser.buffer.pointer.offset(1) as c_int as c_uint,
                    ch0 as c_int as c_uint,
                    ch1 as c_int as c_uint,
                );
                failed = 1;
                break;
            }
            parser.buffer.pointer = parser.buffer.pointer.offset(2);
            parser.unread -= 1;

            k += 1;
        }
        if failed == 0 {
            if yaml_parser_update_buffer(&mut parser, 1) == 0 {
                printf(
                    cstr!("\treader error: %s at %ld\n"),
                    parser.problem,
                    parser.problem_offset as c_long,
                );
                failed = 1;
            } else if *parser.buffer.pointer.offset(0) != b'\0' {
                printf(
                    cstr!("\texpected NUL, found %X (eof=%d, unread=%ld)\n"),
                    *parser.buffer.pointer.offset(0) as c_int as c_uint,
                    parser.eof,
                    parser.unread as c_long,
                );
                failed = 1;
            }
        }
        yaml_parser_delete(&mut parser);
        free(buffer as *mut c_void);
        printf(cstr!("checking a long utf16 sequence: %d fail(s)\n"), failed);
        failed
    }
}

unsafe fn c_main() -> c_int {
    unsafe { check_utf8_sequences() + check_boms() + check_long_utf8() + check_long_utf16() }
}

fn main() {
    ctest::run_void(c_main)
}
