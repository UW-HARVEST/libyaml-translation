//! Translation of `c_src/tests/example-reformatter.c`.

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
use crate::emitter::*;
use crate::parser::*;

unsafe fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        let mut help: c_int = 0;
        let mut canonical: c_int = 0;
        let mut unicode: c_int = 0;
        let mut k: c_int;
        let mut done: c_int = 0;

        let mut parser: yaml_parser_t = core::mem::zeroed();
        let mut emitter: yaml_emitter_t = core::mem::zeroed();
        let mut event: yaml_event_t = core::mem::zeroed();

        /* Clear the objects. */

        memset(
            &mut parser as *mut _ as *mut c_void,
            0,
            core::mem::size_of::<yaml_parser_t>(),
        );
        memset(
            &mut emitter as *mut _ as *mut c_void,
            0,
            core::mem::size_of::<yaml_emitter_t>(),
        );
        memset(
            &mut event as *mut _ as *mut c_void,
            0,
            core::mem::size_of::<yaml_event_t>(),
        );

        /* Analyze command line options. */

        k = 1;
        while k < argc {
            if strcmp(*argv.offset(k as isize), cstr!("-h")) == 0
                || strcmp(*argv.offset(k as isize), cstr!("--help")) == 0
            {
                help = 1;
            } else if strcmp(*argv.offset(k as isize), cstr!("-c")) == 0
                || strcmp(*argv.offset(k as isize), cstr!("--canonical")) == 0
            {
                canonical = 1;
            } else if strcmp(*argv.offset(k as isize), cstr!("-u")) == 0
                || strcmp(*argv.offset(k as isize), cstr!("--unicode")) == 0
            {
                unicode = 1;
            } else {
                fprintf(
                    stderr,
                    cstr!(
                        "Unrecognized option: %s\nTry `%s --help` for more information.\n"
                    ),
                    *argv.offset(k as isize),
                    *argv.offset(0),
                );
                return 1;
            }

            k += 1;
        }

        /* Display the help string. */

        if help != 0 {
            printf(
                cstr!(
                    "%s [--canonical] [--unicode] <input >output\nor\n%s -h | --help\nReformat a YAML stream\n\nOptions:\n-h, --help\t\tdisplay this help and exit\n-c, --canonical\t\toutput in the canonical YAML format\n-u, --unicode\t\toutput unescaped non-ASCII characters\n"
                ),
                *argv.offset(0),
                *argv.offset(0),
            );
            return 0;
        }

        'emitter_error: {
            'parser_error: {
                /* Initialize the parser and emitter objects. */

                if yaml_parser_initialize(&mut parser) == 0 {
                    break 'parser_error;
                }

                if yaml_emitter_initialize(&mut emitter) == 0 {
                    break 'emitter_error;
                }

                /* Set the parser parameters. */

                yaml_parser_set_input_file(&mut parser, stdin);

                /* Set the emitter parameters. */

                yaml_emitter_set_output_file(&mut emitter, stdout);

                yaml_emitter_set_canonical(&mut emitter, canonical);
                yaml_emitter_set_unicode(&mut emitter, unicode);

                /* The main loop. */

                while done == 0 {
                    /* Get the next event. */

                    if yaml_parser_parse(&mut parser, &mut event) == 0 {
                        break 'parser_error;
                    }

                    /* Check if this is the stream end. */

                    if event.type_ == YAML_STREAM_END_EVENT {
                        done = 1;
                    }

                    /* Emit the event. */

                    if yaml_emitter_emit(&mut emitter, &mut event) == 0 {
                        break 'emitter_error;
                    }
                }

                yaml_parser_delete(&mut parser);
                yaml_emitter_delete(&mut emitter);

                return 0;
            }

            /* parser_error: */

            /* Display a parser error message. */

            match parser.error {
                YAML_MEMORY_ERROR => {
                    fprintf(stderr, cstr!("Memory error: Not enough memory for parsing\n"));
                }

                YAML_READER_ERROR => {
                    if parser.problem_value != -1 {
                        fprintf(
                            stderr,
                            cstr!("Reader error: %s: #%X at %ld\n"),
                            parser.problem,
                            parser.problem_value as c_uint,
                            parser.problem_offset as c_long,
                        );
                    } else {
                        fprintf(
                            stderr,
                            cstr!("Reader error: %s at %ld\n"),
                            parser.problem,
                            parser.problem_offset as c_long,
                        );
                    }
                }

                YAML_SCANNER_ERROR => {
                    if !parser.context.is_null() {
                        fprintf(
                            stderr,
                            cstr!(
                                "Scanner error: %s at line %d, column %d\n%s at line %d, column %d\n"
                            ),
                            parser.context,
                            parser.context_mark.line as c_int + 1,
                            parser.context_mark.column as c_int + 1,
                            parser.problem,
                            parser.problem_mark.line as c_int + 1,
                            parser.problem_mark.column as c_int + 1,
                        );
                    } else {
                        fprintf(
                            stderr,
                            cstr!("Scanner error: %s at line %d, column %d\n"),
                            parser.problem,
                            parser.problem_mark.line as c_int + 1,
                            parser.problem_mark.column as c_int + 1,
                        );
                    }
                }

                YAML_PARSER_ERROR => {
                    if !parser.context.is_null() {
                        fprintf(
                            stderr,
                            cstr!(
                                "Parser error: %s at line %d, column %d\n%s at line %d, column %d\n"
                            ),
                            parser.context,
                            parser.context_mark.line as c_int + 1,
                            parser.context_mark.column as c_int + 1,
                            parser.problem,
                            parser.problem_mark.line as c_int + 1,
                            parser.problem_mark.column as c_int + 1,
                        );
                    } else {
                        fprintf(
                            stderr,
                            cstr!("Parser error: %s at line %d, column %d\n"),
                            parser.problem,
                            parser.problem_mark.line as c_int + 1,
                            parser.problem_mark.column as c_int + 1,
                        );
                    }
                }

                _ => {
                    /* Couldn't happen. */
                    fprintf(stderr, cstr!("Internal error\n"));
                }
            }

            yaml_parser_delete(&mut parser);
            yaml_emitter_delete(&mut emitter);

            return 1;
        }

        /* emitter_error: */

        /* Display an emitter error message. */

        match emitter.error {
            YAML_MEMORY_ERROR => {
                fprintf(
                    stderr,
                    cstr!("Memory error: Not enough memory for emitting\n"),
                );
            }

            YAML_WRITER_ERROR => {
                fprintf(stderr, cstr!("Writer error: %s\n"), emitter.problem);
            }

            YAML_EMITTER_ERROR => {
                fprintf(stderr, cstr!("Emitter error: %s\n"), emitter.problem);
            }

            _ => {
                /* Couldn't happen. */
                fprintf(stderr, cstr!("Internal error\n"));
            }
        }

        yaml_parser_delete(&mut parser);
        yaml_emitter_delete(&mut emitter);

        1
    }
}

fn main() {
    ctest::run(c_main)
}
