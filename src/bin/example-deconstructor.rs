//! Translation of `c_src/tests/example-deconstructor.c`.

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
use crate::emitter::yaml_emitter_emit;
use crate::parser::yaml_parser_parse;

unsafe fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        let mut help: c_int = 0;
        let mut canonical: c_int = 0;
        let mut unicode: c_int = 0;
        let mut k: c_int;
        let mut done: c_int = 0;

        let mut parser: yaml_parser_t = core::mem::zeroed();
        let mut emitter: yaml_emitter_t = core::mem::zeroed();
        let mut input_event: yaml_event_t = core::mem::zeroed();
        let mut output_event: yaml_event_t = core::mem::zeroed();

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
            &mut input_event as *mut _ as *mut c_void,
            0,
            core::mem::size_of::<yaml_event_t>(),
        );
        memset(
            &mut output_event as *mut _ as *mut c_void,
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
                    cstr!(concat!(
                        "Unrecognized option: %s\n",
                        "Try `%s --help` for more information.\n"
                    )),
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
                cstr!(concat!(
                    "%s <input\n",
                    "or\n%s -h | --help\nDeconstruct a YAML stream\n\nOptions:\n",
                    "-h, --help\t\tdisplay this help and exit\n",
                    "-c, --canonical\t\toutput in the canonical YAML format\n",
                    "-u, --unicode\t\toutput unescaped non-ASCII characters\n"
                )),
                *argv.offset(0),
                *argv.offset(0),
            );
            return 0;
        }

        /* Initialize the parser and emitter objects. */

        if yaml_parser_initialize(&mut parser) == 0 {
            fprintf(stderr, cstr!("Could not initialize the parser object\n"));
            return 1;
        }

        if yaml_emitter_initialize(&mut emitter) == 0 {
            yaml_parser_delete(&mut parser);
            fprintf(stderr, cstr!("Could not inialize the emitter object\n"));
            return 1;
        }

        /* Set the parser parameters. */

        yaml_parser_set_input_file(&mut parser, stdin);

        /* Set the emitter parameters. */

        yaml_emitter_set_output_file(&mut emitter, stdout);

        yaml_emitter_set_canonical(&mut emitter, canonical);
        yaml_emitter_set_unicode(&mut emitter, unicode);

        'parser_error: {
        'emitter_error: {
        'event_error: {

        /* Create and emit the STREAM-START event. */

        if yaml_stream_start_event_initialize(&mut output_event, YAML_UTF8_ENCODING) == 0 {
            break 'event_error;
        }
        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
            break 'emitter_error;
        }

        /* Create and emit the DOCUMENT-START event. */

        if yaml_document_start_event_initialize(
            &mut output_event,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            0,
        ) == 0
        {
            break 'event_error;
        }
        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
            break 'emitter_error;
        }

        /* Create and emit the SEQUENCE-START event. */

        if yaml_sequence_start_event_initialize(
            &mut output_event,
            core::ptr::null(),
            cstr!("tag:yaml.org,2002:seq") as *const yaml_char_t,
            1,
            YAML_BLOCK_SEQUENCE_STYLE,
        ) == 0
        {
            break 'event_error;
        }
        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
            break 'emitter_error;
        }

        /* Loop through the input events. */

        while done == 0 {
            /* Get the next event. */

            if yaml_parser_parse(&mut parser, &mut input_event) == 0 {
                break 'parser_error;
            }

            /* Check if this is the stream end. */

            if input_event.type_ == YAML_STREAM_END_EVENT {
                done = 1;
            }

            /* Create and emit a MAPPING-START event. */

            if yaml_mapping_start_event_initialize(
                &mut output_event,
                core::ptr::null(),
                cstr!("tag:yaml.org,2002:map") as *const yaml_char_t,
                1,
                YAML_BLOCK_MAPPING_STYLE,
            ) == 0
            {
                break 'event_error;
            }
            if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                break 'emitter_error;
            }

            /* Analyze the event. */

            match input_event.type_ {
                YAML_STREAM_START_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'STREAM-START'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("STREAM-START") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display encoding information. */

                    if input_event.data.stream_start.encoding != 0 {
                        let encoding: yaml_encoding_t = input_event.data.stream_start.encoding;

                        /* Write 'encoding'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("encoding") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the stream encoding. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            (if encoding == YAML_UTF8_ENCODING {
                                cstr!("utf-8")
                            } else if encoding == YAML_UTF16LE_ENCODING {
                                cstr!("utf-16-le")
                            } else if encoding == YAML_UTF16BE_ENCODING {
                                cstr!("utf-16-be")
                            } else {
                                cstr!("unknown")
                            }) as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }
                }

                YAML_STREAM_END_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'STREAM-END'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("STREAM-END") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }
                }

                YAML_DOCUMENT_START_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'DOCUMENT-START'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("DOCUMENT-START") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display the document version numbers. */

                    if !input_event.data.document_start.version_directive.is_null() {
                        let version: *mut yaml_version_directive_t =
                            input_event.data.document_start.version_directive;
                        let mut number: [c_char; 64] = [0; 64];

                        /* Write 'version'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("version") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write '{'. */

                        if yaml_mapping_start_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:map") as *const yaml_char_t,
                            1,
                            YAML_FLOW_MAPPING_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write 'major'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("major") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write a number. */

                        sprintf(number.as_mut_ptr(), cstr!("%d"), (*version).major);
                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:int") as *const yaml_char_t,
                            number.as_mut_ptr() as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write 'minor'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("minor") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write a number. */

                        sprintf(number.as_mut_ptr(), cstr!("%d"), (*version).minor);
                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:int") as *const yaml_char_t,
                            number.as_mut_ptr() as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write '}'. */

                        if yaml_mapping_end_event_initialize(&mut output_event) == 0 {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }

                    /* Display the document tag directives. */

                    if input_event.data.document_start.tag_directives.start
                        != input_event.data.document_start.tag_directives.end
                    {
                        let mut tag: *mut yaml_tag_directive_t;

                        /* Write 'tags'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("tags") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Start a block sequence. */

                        if yaml_sequence_start_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:seq") as *const yaml_char_t,
                            1,
                            YAML_BLOCK_SEQUENCE_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        tag = input_event.data.document_start.tag_directives.start;
                        while tag != input_event.data.document_start.tag_directives.end {
                            /* Write '{'. */

                            if yaml_mapping_start_event_initialize(
                                &mut output_event,
                                core::ptr::null(),
                                cstr!("tag:yaml.org,2002:map") as *const yaml_char_t,
                                1,
                                YAML_FLOW_MAPPING_STYLE,
                            ) == 0
                            {
                                break 'event_error;
                            }
                            if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                                break 'emitter_error;
                            }

                            /* Write 'handle'. */

                            if yaml_scalar_event_initialize(
                                &mut output_event,
                                core::ptr::null(),
                                cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                                cstr!("handle") as *const yaml_char_t,
                                -1,
                                1,
                                1,
                                YAML_PLAIN_SCALAR_STYLE,
                            ) == 0
                            {
                                break 'event_error;
                            }
                            if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                                break 'emitter_error;
                            }

                            /* Write the tag directive handle. */

                            if yaml_scalar_event_initialize(
                                &mut output_event,
                                core::ptr::null(),
                                cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                                (*tag).handle,
                                -1,
                                0,
                                1,
                                YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                            ) == 0
                            {
                                break 'event_error;
                            }
                            if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                                break 'emitter_error;
                            }

                            /* Write 'prefix'. */

                            if yaml_scalar_event_initialize(
                                &mut output_event,
                                core::ptr::null(),
                                cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                                cstr!("prefix") as *const yaml_char_t,
                                -1,
                                1,
                                1,
                                YAML_PLAIN_SCALAR_STYLE,
                            ) == 0
                            {
                                break 'event_error;
                            }
                            if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                                break 'emitter_error;
                            }

                            /* Write the tag directive prefix. */

                            if yaml_scalar_event_initialize(
                                &mut output_event,
                                core::ptr::null(),
                                cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                                (*tag).prefix,
                                -1,
                                0,
                                1,
                                YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                            ) == 0
                            {
                                break 'event_error;
                            }
                            if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                                break 'emitter_error;
                            }

                            /* Write '}'. */

                            if yaml_mapping_end_event_initialize(&mut output_event) == 0 {
                                break 'event_error;
                            }
                            if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                                break 'emitter_error;
                            }

                            tag = tag.offset(1);
                        }

                        /* End a block sequence. */

                        if yaml_sequence_end_event_initialize(&mut output_event) == 0 {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }

                    /* Write 'implicit'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("implicit") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write if the document is implicit. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:bool") as *const yaml_char_t,
                        (if input_event.data.document_start.implicit != 0 {
                            cstr!("true")
                        } else {
                            cstr!("false")
                        }) as *const yaml_char_t,
                        -1,
                        1,
                        0,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }
                }

                YAML_DOCUMENT_END_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'DOCUMENT-END'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("DOCUMENT-END") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'implicit'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("implicit") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write if the document is implicit. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:bool") as *const yaml_char_t,
                        (if input_event.data.document_end.implicit != 0 {
                            cstr!("true")
                        } else {
                            cstr!("false")
                        }) as *const yaml_char_t,
                        -1,
                        1,
                        0,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }
                }

                YAML_ALIAS_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'ALIAS'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("ALIAS") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'anchor'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("anchor") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write the alias anchor. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        input_event.data.alias.anchor,
                        -1,
                        0,
                        1,
                        YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }
                }

                YAML_SCALAR_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'SCALAR'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("SCALAR") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display the scalar anchor. */

                    if !input_event.data.scalar.anchor.is_null() {
                        /* Write 'anchor'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("anchor") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the scalar anchor. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            input_event.data.scalar.anchor,
                            -1,
                            0,
                            1,
                            YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }

                    /* Display the scalar tag. */

                    if !input_event.data.scalar.tag.is_null() {
                        /* Write 'tag'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("tag") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the scalar tag. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            input_event.data.scalar.tag,
                            -1,
                            0,
                            1,
                            YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }

                    /* Display the scalar value. */

                    /* Write 'value'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("value") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write the scalar value. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        input_event.data.scalar.value,
                        input_event.data.scalar.length as c_int,
                        0,
                        1,
                        YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display if the scalar tag is implicit. */

                    /* Write 'implicit'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("implicit") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write '{'. */

                    if yaml_mapping_start_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:map") as *const yaml_char_t,
                        1,
                        YAML_FLOW_MAPPING_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'plain'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("plain") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write if the scalar is implicit in the plain style. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:bool") as *const yaml_char_t,
                        (if input_event.data.scalar.plain_implicit != 0 {
                            cstr!("true")
                        } else {
                            cstr!("false")
                        }) as *const yaml_char_t,
                        -1,
                        1,
                        0,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'quoted'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("non-plain") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write if the scalar is implicit in a non-plain style. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:bool") as *const yaml_char_t,
                        (if input_event.data.scalar.quoted_implicit != 0 {
                            cstr!("true")
                        } else {
                            cstr!("false")
                        }) as *const yaml_char_t,
                        -1,
                        1,
                        0,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write '}'. */

                    if yaml_mapping_end_event_initialize(&mut output_event) == 0 {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display the style information. */

                    if input_event.data.scalar.style != 0 {
                        let style: yaml_scalar_style_t = input_event.data.scalar.style;

                        /* Write 'style'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("style") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the scalar style. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            (if style == YAML_PLAIN_SCALAR_STYLE {
                                cstr!("plain")
                            } else if style == YAML_SINGLE_QUOTED_SCALAR_STYLE {
                                cstr!("single-quoted")
                            } else if style == YAML_DOUBLE_QUOTED_SCALAR_STYLE {
                                cstr!("double-quoted")
                            } else if style == YAML_LITERAL_SCALAR_STYLE {
                                cstr!("literal")
                            } else if style == YAML_FOLDED_SCALAR_STYLE {
                                cstr!("folded")
                            } else {
                                cstr!("unknown")
                            }) as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }
                }

                YAML_SEQUENCE_START_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'SEQUENCE-START'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("SEQUENCE-START") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display the sequence anchor. */

                    if !input_event.data.sequence_start.anchor.is_null() {
                        /* Write 'anchor'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("anchor") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the sequence anchor. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            input_event.data.sequence_start.anchor,
                            -1,
                            0,
                            1,
                            YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }

                    /* Display the sequence tag. */

                    if !input_event.data.sequence_start.tag.is_null() {
                        /* Write 'tag'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("tag") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the sequence tag. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            input_event.data.sequence_start.tag,
                            -1,
                            0,
                            1,
                            YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }

                    /* Write 'implicit'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("implicit") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write if the sequence tag is implicit. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:bool") as *const yaml_char_t,
                        (if input_event.data.sequence_start.implicit != 0 {
                            cstr!("true")
                        } else {
                            cstr!("false")
                        }) as *const yaml_char_t,
                        -1,
                        1,
                        0,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display the style information. */

                    if input_event.data.sequence_start.style != 0 {
                        let style: yaml_sequence_style_t = input_event.data.sequence_start.style;

                        /* Write 'style'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("style") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the scalar style. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            (if style == YAML_BLOCK_SEQUENCE_STYLE {
                                cstr!("block")
                            } else if style == YAML_FLOW_SEQUENCE_STYLE {
                                cstr!("flow")
                            } else {
                                cstr!("unknown")
                            }) as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }
                }

                YAML_SEQUENCE_END_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'SEQUENCE-END'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("SEQUENCE-END") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }
                }

                YAML_MAPPING_START_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'MAPPING-START'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("MAPPING-START") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display the mapping anchor. */

                    if !input_event.data.mapping_start.anchor.is_null() {
                        /* Write 'anchor'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("anchor") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the mapping anchor. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            input_event.data.mapping_start.anchor,
                            -1,
                            0,
                            1,
                            YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }

                    /* Display the mapping tag. */

                    if !input_event.data.mapping_start.tag.is_null() {
                        /* Write 'tag'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("tag") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the mapping tag. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            input_event.data.mapping_start.tag,
                            -1,
                            0,
                            1,
                            YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }

                    /* Write 'implicit'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("implicit") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write if the mapping tag is implicit. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:bool") as *const yaml_char_t,
                        (if input_event.data.mapping_start.implicit != 0 {
                            cstr!("true")
                        } else {
                            cstr!("false")
                        }) as *const yaml_char_t,
                        -1,
                        1,
                        0,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Display the style information. */

                    if input_event.data.mapping_start.style != 0 {
                        let style: yaml_mapping_style_t = input_event.data.mapping_start.style;

                        /* Write 'style'. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            cstr!("style") as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }

                        /* Write the scalar style. */

                        if yaml_scalar_event_initialize(
                            &mut output_event,
                            core::ptr::null(),
                            cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                            (if style == YAML_BLOCK_MAPPING_STYLE {
                                cstr!("block")
                            } else if style == YAML_FLOW_MAPPING_STYLE {
                                cstr!("flow")
                            } else {
                                cstr!("unknown")
                            }) as *const yaml_char_t,
                            -1,
                            1,
                            1,
                            YAML_PLAIN_SCALAR_STYLE,
                        ) == 0
                        {
                            break 'event_error;
                        }
                        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                            break 'emitter_error;
                        }
                    }
                }

                YAML_MAPPING_END_EVENT => {
                    /* Write 'type'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("type") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }

                    /* Write 'MAPPING-END'. */

                    if yaml_scalar_event_initialize(
                        &mut output_event,
                        core::ptr::null(),
                        cstr!("tag:yaml.org,2002:str") as *const yaml_char_t,
                        cstr!("MAPPING-END") as *const yaml_char_t,
                        -1,
                        1,
                        1,
                        YAML_PLAIN_SCALAR_STYLE,
                    ) == 0
                    {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                        break 'emitter_error;
                    }
                }

                _ => {
                    /* It couldn't really happen. */
                }
            }

            /* Delete the event object. */

            yaml_event_delete(&mut input_event);

            /* Create and emit a MAPPING-END event. */

            if yaml_mapping_end_event_initialize(&mut output_event) == 0 {
                break 'event_error;
            }
            if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
                break 'emitter_error;
            }
        }

        /* Create and emit the SEQUENCE-END event. */

        if yaml_sequence_end_event_initialize(&mut output_event) == 0 {
            break 'event_error;
        }
        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
            break 'emitter_error;
        }

        /* Create and emit the DOCUMENT-END event. */

        if yaml_document_end_event_initialize(&mut output_event, 0) == 0 {
            break 'event_error;
        }
        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
            break 'emitter_error;
        }

        /* Create and emit the STREAM-END event. */

        if yaml_stream_end_event_initialize(&mut output_event) == 0 {
            break 'event_error;
        }
        if yaml_emitter_emit(&mut emitter, &mut output_event) == 0 {
            break 'emitter_error;
        }

        yaml_parser_delete(&mut parser);
        yaml_emitter_delete(&mut emitter);

        return 0;

        } /* 'event_error */

        /* event_error: */

        fprintf(
            stderr,
            cstr!("Memory error: Not enough memory for creating an event\n"),
        );

        yaml_event_delete(&mut input_event);
        yaml_parser_delete(&mut parser);
        yaml_emitter_delete(&mut emitter);

        return 1;

        } /* 'emitter_error */

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

        yaml_event_delete(&mut input_event);
        yaml_parser_delete(&mut parser);
        yaml_emitter_delete(&mut emitter);

        return 1;

        } /* 'parser_error */

        /* parser_error: */

        /* Display a parser error message. */

        match parser.error {
            YAML_MEMORY_ERROR => {
                fprintf(
                    stderr,
                    cstr!("Memory error: Not enough memory for parsing\n"),
                );
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
                        cstr!(concat!(
                            "Scanner error: %s at line %d, column %d\n",
                            "%s at line %d, column %d\n"
                        )),
                        parser.context,
                        (parser.context_mark.line as c_int) + 1,
                        (parser.context_mark.column as c_int) + 1,
                        parser.problem,
                        (parser.problem_mark.line as c_int) + 1,
                        (parser.problem_mark.column as c_int) + 1,
                    );
                } else {
                    fprintf(
                        stderr,
                        cstr!("Scanner error: %s at line %d, column %d\n"),
                        parser.problem,
                        (parser.problem_mark.line as c_int) + 1,
                        (parser.problem_mark.column as c_int) + 1,
                    );
                }
            }

            YAML_PARSER_ERROR => {
                if !parser.context.is_null() {
                    fprintf(
                        stderr,
                        cstr!(concat!(
                            "Parser error: %s at line %d, column %d\n",
                            "%s at line %d, column %d\n"
                        )),
                        parser.context,
                        (parser.context_mark.line as c_int) + 1,
                        (parser.context_mark.column as c_int) + 1,
                        parser.problem,
                        (parser.problem_mark.line as c_int) + 1,
                        (parser.problem_mark.column as c_int) + 1,
                    );
                } else {
                    fprintf(
                        stderr,
                        cstr!("Parser error: %s at line %d, column %d\n"),
                        parser.problem,
                        (parser.problem_mark.line as c_int) + 1,
                        (parser.problem_mark.column as c_int) + 1,
                    );
                }
            }

            _ => {
                /* Couldn't happen. */
                fprintf(stderr, cstr!("Internal error\n"));
            }
        }

        yaml_event_delete(&mut input_event);
        yaml_parser_delete(&mut parser);
        yaml_emitter_delete(&mut emitter);

        1
    }
}

fn main() {
    ctest::run(c_main)
}
