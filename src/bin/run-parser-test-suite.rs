//! Translation of `c_src/tests/run-parser-test-suite.c`.

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
        let mut input: *mut FILE = core::ptr::null_mut();
        let mut parser: yaml_parser_t = core::mem::zeroed();
        let mut event: yaml_event_t = core::mem::zeroed();
        let mut flow: c_int = -1; /* default no flow style collections */
        let mut i: c_int = 0;
        let mut foundfile: c_int = 0;
        let mut max_level: c_int = 0;
        let mut output: *mut c_char = core::ptr::null_mut();

        i = 1;
        while i < argc {
            if strncmp(*argv.offset(i as isize), cstr!("--max-level"), 11) == 0 {
                i += 1;
                max_level = strtol(*argv.offset(i as isize), &mut output, 10) as c_int;
                yaml_set_max_nest_level(max_level);
            } else if strncmp(*argv.offset(i as isize), cstr!("--flow"), 6) == 0 {
                if i + 1 == argc {
                    return usage(1);
                }
                i += 1;
                if strncmp(*argv.offset(i as isize), cstr!("keep"), 4) == 0 {
                    flow = 0;
                } else if strncmp(*argv.offset(i as isize), cstr!("on"), 2) == 0 {
                    flow = 1;
                } else if strncmp(*argv.offset(i as isize), cstr!("off"), 3) == 0 {
                    flow = -1;
                } else {
                    return usage(1);
                }
            } else if strncmp(*argv.offset(i as isize), cstr!("--help"), 6) == 0 {
                return usage(0);
            } else if strncmp(*argv.offset(i as isize), cstr!("-h"), 2) == 0 {
                return usage(0);
            } else if foundfile == 0 {
                input = fopen(*argv.offset(i as isize), cstr!("rb"));
                foundfile = 1;
            } else {
                return usage(1);
            }

            i += 1;
        }
        if foundfile == 0 {
            input = stdin;
        }
        c_assert!(input);

        if yaml_parser_initialize(&mut parser) == 0 {
            fprintf(stderr, cstr!("Could not initialize the parser object\n"));
            return 1;
        }
        yaml_parser_set_input_file(&mut parser, input);

        loop {
            let type_: yaml_event_type_t;
            if yaml_parser_parse(&mut parser, &mut event) == 0 {
                if parser.problem_mark.line != 0 || parser.problem_mark.column != 0 {
                    fprintf(
                        stderr,
                        cstr!("Parse error: %s\nLine: %lu Column: %lu\n"),
                        parser.problem,
                        (parser.problem_mark.line as u64) + 1,
                        (parser.problem_mark.column as u64) + 1,
                    );
                } else {
                    fprintf(stderr, cstr!("Parse error: %s\n"), parser.problem);
                }
                return 1;
            }
            type_ = event.type_;

            if type_ == YAML_NO_EVENT {
                printf(cstr!("???\n"));
            } else if type_ == YAML_STREAM_START_EVENT {
                printf(cstr!("+STR\n"));
            } else if type_ == YAML_STREAM_END_EVENT {
                printf(cstr!("-STR\n"));
            } else if type_ == YAML_DOCUMENT_START_EVENT {
                printf(cstr!("+DOC"));
                if event.data.document_start.implicit == 0 {
                    printf(cstr!(" ---"));
                }
                printf(cstr!("\n"));
            } else if type_ == YAML_DOCUMENT_END_EVENT {
                printf(cstr!("-DOC"));
                if event.data.document_end.implicit == 0 {
                    printf(cstr!(" ..."));
                }
                printf(cstr!("\n"));
            } else if type_ == YAML_MAPPING_START_EVENT {
                printf(cstr!("+MAP"));
                if flow == 0 && event.data.mapping_start.style == YAML_FLOW_MAPPING_STYLE {
                    printf(cstr!(" {}"));
                } else if flow == 1 {
                    printf(cstr!(" {}"));
                }
                if !event.data.mapping_start.anchor.is_null() {
                    printf(
                        cstr!(" &%s"),
                        event.data.mapping_start.anchor as *const c_char,
                    );
                }
                if !event.data.mapping_start.tag.is_null() {
                    printf(
                        cstr!(" <%s>"),
                        event.data.mapping_start.tag as *const c_char,
                    );
                }
                printf(cstr!("\n"));
            } else if type_ == YAML_MAPPING_END_EVENT {
                printf(cstr!("-MAP\n"));
            } else if type_ == YAML_SEQUENCE_START_EVENT {
                printf(cstr!("+SEQ"));
                if flow == 0 && event.data.sequence_start.style == YAML_FLOW_SEQUENCE_STYLE {
                    printf(cstr!(" []"));
                } else if flow == 1 {
                    printf(cstr!(" []"));
                }
                if !event.data.sequence_start.anchor.is_null() {
                    printf(
                        cstr!(" &%s"),
                        event.data.sequence_start.anchor as *const c_char,
                    );
                }
                if !event.data.sequence_start.tag.is_null() {
                    printf(
                        cstr!(" <%s>"),
                        event.data.sequence_start.tag as *const c_char,
                    );
                }
                printf(cstr!("\n"));
            } else if type_ == YAML_SEQUENCE_END_EVENT {
                printf(cstr!("-SEQ\n"));
            } else if type_ == YAML_SCALAR_EVENT {
                printf(cstr!("=VAL"));
                if !event.data.scalar.anchor.is_null() {
                    printf(cstr!(" &%s"), event.data.scalar.anchor as *const c_char);
                }
                if !event.data.scalar.tag.is_null() {
                    printf(cstr!(" <%s>"), event.data.scalar.tag as *const c_char);
                }
                /* switch (event.data.scalar.style) */
                let style: yaml_scalar_style_t = event.data.scalar.style;
                if style == YAML_PLAIN_SCALAR_STYLE {
                    printf(cstr!(" :"));
                } else if style == YAML_SINGLE_QUOTED_SCALAR_STYLE {
                    printf(cstr!(" '"));
                } else if style == YAML_DOUBLE_QUOTED_SCALAR_STYLE {
                    printf(cstr!(" \""));
                } else if style == YAML_LITERAL_SCALAR_STYLE {
                    printf(cstr!(" |"));
                } else if style == YAML_FOLDED_SCALAR_STYLE {
                    printf(cstr!(" >"));
                } else if style == YAML_ANY_SCALAR_STYLE {
                    abort();
                }
                print_escaped(event.data.scalar.value, event.data.scalar.length);
                printf(cstr!("\n"));
            } else if type_ == YAML_ALIAS_EVENT {
                printf(cstr!("=ALI *%s\n"), event.data.alias.anchor as *const c_char);
            } else {
                abort();
            }

            yaml_event_delete(&mut event);

            if type_ == YAML_STREAM_END_EVENT {
                break;
            }
        }

        c_assert!(fclose(input) == 0);
        yaml_parser_delete(&mut parser);
        fflush(stdout);

        0
    }
}

unsafe fn print_escaped(str_: *mut yaml_char_t, length: size_t) {
    unsafe {
        let mut i: c_int;
        let mut c: c_char;

        i = 0;
        while (i as size_t) < length {
            c = *(str_.offset(i as isize)) as c_char;
            if c == ('\\' as c_char) {
                printf(cstr!("\\\\"));
            } else if c == ('\0' as c_char) {
                printf(cstr!("\\0"));
            } else if c == (0x08 as c_char) {
                /* '\b' */
                printf(cstr!("\\b"));
            } else if c == ('\n' as c_char) {
                printf(cstr!("\\n"));
            } else if c == ('\r' as c_char) {
                printf(cstr!("\\r"));
            } else if c == ('\t' as c_char) {
                printf(cstr!("\\t"));
            } else {
                printf(cstr!("%c"), c as c_int);
            }

            i += 1;
        }
    }
}

unsafe fn usage(ret: c_int) -> c_int {
    unsafe {
        fprintf(
            stderr,
            cstr!("Usage: libyaml-parser [--flow (on|off|keep)] [<input-file>]\n"),
        );
        ret
    }
}

fn main() {
    ctest::run(c_main)
}
