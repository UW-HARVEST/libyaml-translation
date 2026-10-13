//! Translation of `c_src/tests/example-deconstructor-alt.c`.

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
use crate::api_objects::*;
use crate::ctest::*;
use crate::dumper::*;
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
        let mut input_event: yaml_event_t = core::mem::zeroed();
        let mut output_document: yaml_document_t = core::mem::zeroed();

        let mut root: c_int;

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
            &mut output_document as *mut _ as *mut c_void,
            0,
            core::mem::size_of::<yaml_document_t>(),
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
                cstr!("%s <input\nor\n%s -h | --help\nDeconstruct a YAML stream\n\nOptions:\n-h, --help\t\tdisplay this help and exit\n-c, --canonical\t\toutput in the canonical YAML format\n-u, --unicode\t\toutput unescaped non-ASCII characters\n"),
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
                'document_error: {
                    /* Create and emit the STREAM-START event. */

                    if yaml_emitter_open(&mut emitter) == 0 {
                        break 'emitter_error;
                    }

                    /* Create a output_document object. */

                    if yaml_document_initialize(
                        &mut output_document,
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        0,
                        0,
                    ) == 0
                    {
                        break 'document_error;
                    }

                    /* Create the root sequence. */

                    root = yaml_document_add_sequence(
                        &mut output_document,
                        core::ptr::null(),
                        YAML_BLOCK_SEQUENCE_STYLE,
                    );
                    if root == 0 {
                        break 'document_error;
                    }

                    /* Loop through the input events. */

                    while done == 0 {
                        let mut properties: c_int = 0;
                        let mut key: c_int = 0;
                        let mut value: c_int = 0;
                        let mut map: c_int = 0;
                        let mut seq: c_int = 0;

                        /* Get the next event. */

                        if yaml_parser_parse(&mut parser, &mut input_event) == 0 {
                            break 'parser_error;
                        }

                        /* Check if this is the stream end. */

                        if input_event.type_ == YAML_STREAM_END_EVENT {
                            done = 1;
                        }

                        /* Create a mapping node and attach it to the root sequence. */

                        properties = yaml_document_add_mapping(
                            &mut output_document,
                            core::ptr::null(),
                            YAML_BLOCK_MAPPING_STYLE,
                        );
                        if properties == 0 {
                            break 'document_error;
                        }
                        if yaml_document_append_sequence_item(
                            &mut output_document,
                            root,
                            properties,
                        ) == 0
                        {
                            break 'document_error;
                        }

                        /* Analyze the event. */

                        match input_event.type_ {
                            YAML_STREAM_START_EVENT => {
                                /* Add 'type': 'STREAM-START'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("STREAM-START") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Add 'encoding': <encoding>. */

                                if input_event.data.stream_start.encoding != 0 {
                                    let encoding: yaml_encoding_t =
                                        input_event.data.stream_start.encoding;

                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("encoding") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
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
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }
                            }

                            YAML_STREAM_END_EVENT => {
                                /* Add 'type': 'STREAM-END'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("STREAM-END") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }
                            }

                            YAML_DOCUMENT_START_EVENT => {
                                /* Add 'type': 'DOCUMENT-START'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("DOCUMENT-START") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Display the output_document version numbers. */

                                if !input_event.data.document_start.version_directive.is_null() {
                                    let version: *mut yaml_version_directive_t =
                                        input_event.data.document_start.version_directive;
                                    let mut number: [c_char; 64] = [0; 64];

                                    /* Add 'version': {}. */

                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("version") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    map = yaml_document_add_mapping(
                                        &mut output_document,
                                        core::ptr::null(),
                                        YAML_FLOW_MAPPING_STYLE,
                                    );
                                    if map == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        map,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }

                                    /* Add 'major': <number>. */

                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("major") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    sprintf(number.as_mut_ptr(), cstr!("%d"), (*version).major);
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        YAML_INT_TAG.as_ptr() as *const yaml_char_t,
                                        number.as_mut_ptr() as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        map,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }

                                    /* Add 'minor': <number>. */

                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("minor") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    sprintf(number.as_mut_ptr(), cstr!("%d"), (*version).minor);
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        YAML_INT_TAG.as_ptr() as *const yaml_char_t,
                                        number.as_mut_ptr() as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        map,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }

                                /* Display the output_document tag directives. */

                                if input_event.data.document_start.tag_directives.start
                                    != input_event.data.document_start.tag_directives.end
                                {
                                    let mut tag: *mut yaml_tag_directive_t;

                                    /* Add 'tags': []. */

                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("tags") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    seq = yaml_document_add_sequence(
                                        &mut output_document,
                                        core::ptr::null(),
                                        YAML_BLOCK_SEQUENCE_STYLE,
                                    );
                                    if seq == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        seq,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }

                                    tag = input_event.data.document_start.tag_directives.start;
                                    while tag != input_event.data.document_start.tag_directives.end
                                    {
                                        /* Add {}. */

                                        map = yaml_document_add_mapping(
                                            &mut output_document,
                                            core::ptr::null(),
                                            YAML_FLOW_MAPPING_STYLE,
                                        );
                                        if map == 0 {
                                            break 'document_error;
                                        }
                                        if yaml_document_append_sequence_item(
                                            &mut output_document,
                                            seq,
                                            map,
                                        ) == 0
                                        {
                                            break 'document_error;
                                        }

                                        /* Add 'handle': <handle>. */

                                        key = yaml_document_add_scalar(
                                            &mut output_document,
                                            core::ptr::null(),
                                            cstr!("handle") as *const yaml_char_t,
                                            -1,
                                            YAML_PLAIN_SCALAR_STYLE,
                                        );
                                        if key == 0 {
                                            break 'document_error;
                                        }
                                        value = yaml_document_add_scalar(
                                            &mut output_document,
                                            core::ptr::null(),
                                            (*tag).handle as *const yaml_char_t,
                                            -1,
                                            YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                        );
                                        if value == 0 {
                                            break 'document_error;
                                        }
                                        if yaml_document_append_mapping_pair(
                                            &mut output_document,
                                            map,
                                            key,
                                            value,
                                        ) == 0
                                        {
                                            break 'document_error;
                                        }

                                        /* Add 'prefix': <prefix>. */

                                        key = yaml_document_add_scalar(
                                            &mut output_document,
                                            core::ptr::null(),
                                            cstr!("prefix") as *const yaml_char_t,
                                            -1,
                                            YAML_PLAIN_SCALAR_STYLE,
                                        );
                                        if key == 0 {
                                            break 'document_error;
                                        }
                                        value = yaml_document_add_scalar(
                                            &mut output_document,
                                            core::ptr::null(),
                                            (*tag).prefix as *const yaml_char_t,
                                            -1,
                                            YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                        );
                                        if value == 0 {
                                            break 'document_error;
                                        }
                                        if yaml_document_append_mapping_pair(
                                            &mut output_document,
                                            map,
                                            key,
                                            value,
                                        ) == 0
                                        {
                                            break 'document_error;
                                        }

                                        tag = tag.offset(1);
                                    }
                                }

                                /* Add 'implicit': <flag>. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("implicit") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    YAML_BOOL_TAG.as_ptr() as *const yaml_char_t,
                                    (if input_event.data.document_start.implicit != 0 {
                                        cstr!("true")
                                    } else {
                                        cstr!("false")
                                    }) as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }
                            }

                            YAML_DOCUMENT_END_EVENT => {
                                /* Add 'type': 'DOCUMENT-END'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("DOCUMENT-END") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Add 'implicit': <flag>. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("implicit") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    YAML_BOOL_TAG.as_ptr() as *const yaml_char_t,
                                    (if input_event.data.document_end.implicit != 0 {
                                        cstr!("true")
                                    } else {
                                        cstr!("false")
                                    }) as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }
                            }

                            YAML_ALIAS_EVENT => {
                                /* Add 'type': 'ALIAS'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("ALIAS") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Add 'anchor': <anchor>. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("anchor") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    input_event.data.alias.anchor as *const yaml_char_t,
                                    -1,
                                    YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }
                            }

                            YAML_SCALAR_EVENT => {
                                /* Add 'type': 'SCALAR'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("SCALAR") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Add 'anchor': <anchor>. */

                                if !input_event.data.scalar.anchor.is_null() {
                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("anchor") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        input_event.data.scalar.anchor as *const yaml_char_t,
                                        -1,
                                        YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }

                                /* Add 'tag': <tag>. */

                                if !input_event.data.scalar.tag.is_null() {
                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("tag") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        input_event.data.scalar.tag as *const yaml_char_t,
                                        -1,
                                        YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }

                                /* Add 'value': <value>. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("value") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    input_event.data.scalar.value as *const yaml_char_t,
                                    input_event.data.scalar.length as c_int,
                                    YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Display if the scalar tag is implicit. */

                                /* Add 'implicit': {} */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("version") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                map = yaml_document_add_mapping(
                                    &mut output_document,
                                    core::ptr::null(),
                                    YAML_FLOW_MAPPING_STYLE,
                                );
                                if map == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    map,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Add 'plain': <flag>. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("plain") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    YAML_BOOL_TAG.as_ptr() as *const yaml_char_t,
                                    (if input_event.data.scalar.plain_implicit != 0 {
                                        cstr!("true")
                                    } else {
                                        cstr!("false")
                                    }) as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    map,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Add 'quoted': <flag>. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("quoted") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    YAML_BOOL_TAG.as_ptr() as *const yaml_char_t,
                                    (if input_event.data.scalar.quoted_implicit != 0 {
                                        cstr!("true")
                                    } else {
                                        cstr!("false")
                                    }) as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    map,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Display the style information. */

                                if input_event.data.scalar.style != 0 {
                                    let style: yaml_scalar_style_t =
                                        input_event.data.scalar.style;

                                    /* Add 'style': <style>. */

                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("style") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
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
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }
                            }

                            YAML_SEQUENCE_START_EVENT => {
                                /* Add 'type': 'SEQUENCE-START'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("SEQUENCE-START") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Add 'anchor': <anchor>. */

                                if !input_event.data.sequence_start.anchor.is_null() {
                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("anchor") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        input_event.data.sequence_start.anchor
                                            as *const yaml_char_t,
                                        -1,
                                        YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }

                                /* Add 'tag': <tag>. */

                                if !input_event.data.sequence_start.tag.is_null() {
                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("tag") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        input_event.data.sequence_start.tag as *const yaml_char_t,
                                        -1,
                                        YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }

                                /* Add 'implicit': <flag>. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("implicit") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    YAML_BOOL_TAG.as_ptr() as *const yaml_char_t,
                                    (if input_event.data.sequence_start.implicit != 0 {
                                        cstr!("true")
                                    } else {
                                        cstr!("false")
                                    }) as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Display the style information. */

                                if input_event.data.sequence_start.style != 0 {
                                    let style: yaml_sequence_style_t =
                                        input_event.data.sequence_start.style;

                                    /* Add 'style': <style>. */

                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("style") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        (if style == YAML_BLOCK_SEQUENCE_STYLE {
                                            cstr!("block")
                                        } else if style == YAML_FLOW_SEQUENCE_STYLE {
                                            cstr!("flow")
                                        } else {
                                            cstr!("unknown")
                                        }) as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }
                            }

                            YAML_SEQUENCE_END_EVENT => {
                                /* Add 'type': 'SEQUENCE-END'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("SEQUENCE-END") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }
                            }

                            YAML_MAPPING_START_EVENT => {
                                /* Add 'type': 'MAPPING-START'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("MAPPING-START") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Add 'anchor': <anchor>. */

                                if !input_event.data.mapping_start.anchor.is_null() {
                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("anchor") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        input_event.data.mapping_start.anchor
                                            as *const yaml_char_t,
                                        -1,
                                        YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }

                                /* Add 'tag': <tag>. */

                                if !input_event.data.mapping_start.tag.is_null() {
                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("tag") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        input_event.data.mapping_start.tag as *const yaml_char_t,
                                        -1,
                                        YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }

                                /* Add 'implicit': <flag>. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("implicit") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    YAML_BOOL_TAG.as_ptr() as *const yaml_char_t,
                                    (if input_event.data.mapping_start.implicit != 0 {
                                        cstr!("true")
                                    } else {
                                        cstr!("false")
                                    }) as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }

                                /* Display the style information. */

                                if input_event.data.mapping_start.style != 0 {
                                    let style: yaml_mapping_style_t =
                                        input_event.data.mapping_start.style;

                                    /* Add 'style': <style>. */

                                    key = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        cstr!("style") as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if key == 0 {
                                        break 'document_error;
                                    }
                                    value = yaml_document_add_scalar(
                                        &mut output_document,
                                        core::ptr::null(),
                                        (if style == YAML_BLOCK_MAPPING_STYLE {
                                            cstr!("block")
                                        } else if style == YAML_FLOW_MAPPING_STYLE {
                                            cstr!("flow")
                                        } else {
                                            cstr!("unknown")
                                        }) as *const yaml_char_t,
                                        -1,
                                        YAML_PLAIN_SCALAR_STYLE,
                                    );
                                    if value == 0 {
                                        break 'document_error;
                                    }
                                    if yaml_document_append_mapping_pair(
                                        &mut output_document,
                                        properties,
                                        key,
                                        value,
                                    ) == 0
                                    {
                                        break 'document_error;
                                    }
                                }
                            }

                            YAML_MAPPING_END_EVENT => {
                                /* Add 'type': 'MAPPING-END'. */

                                key = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("type") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if key == 0 {
                                    break 'document_error;
                                }
                                value = yaml_document_add_scalar(
                                    &mut output_document,
                                    core::ptr::null(),
                                    cstr!("MAPPING-END") as *const yaml_char_t,
                                    -1,
                                    YAML_PLAIN_SCALAR_STYLE,
                                );
                                if value == 0 {
                                    break 'document_error;
                                }
                                if yaml_document_append_mapping_pair(
                                    &mut output_document,
                                    properties,
                                    key,
                                    value,
                                ) == 0
                                {
                                    break 'document_error;
                                }
                            }

                            _ => {
                                /* It couldn't really happen. */
                            }
                        }

                        /* Delete the event object. */

                        yaml_event_delete(&mut input_event);
                    }

                    if yaml_emitter_dump(&mut emitter, &mut output_document) == 0 {
                        break 'emitter_error;
                    }
                    if yaml_emitter_close(&mut emitter) == 0 {
                        break 'emitter_error;
                    }

                    yaml_parser_delete(&mut parser);
                    yaml_emitter_delete(&mut emitter);

                    return 0;
                }

                /* document_error: */

                fprintf(
                    stderr,
                    cstr!("Memory error: Not enough memory for creating a document\n"),
                );

                yaml_event_delete(&mut input_event);
                yaml_document_delete(&mut output_document);
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

            yaml_event_delete(&mut input_event);
            yaml_document_delete(&mut output_document);
            yaml_parser_delete(&mut parser);
            yaml_emitter_delete(&mut emitter);

            return 1;
        }

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
                        cstr!("Reader error: %s: #%X at %zd\n"),
                        parser.problem,
                        parser.problem_value as c_uint,
                        parser.problem_offset as size_t,
                    );
                } else {
                    fprintf(
                        stderr,
                        cstr!("Reader error: %s at %zd\n"),
                        parser.problem,
                        parser.problem_offset as size_t,
                    );
                }
            }

            YAML_SCANNER_ERROR => {
                if !parser.context.is_null() {
                    fprintf(
                        stderr,
                        cstr!("Scanner error: %s at line %lu, column %lu\n%s at line %lu, column %lu\n"),
                        parser.context,
                        (parser.context_mark.line as u64) + 1,
                        (parser.context_mark.column as u64) + 1,
                        parser.problem,
                        (parser.problem_mark.line as u64) + 1,
                        (parser.problem_mark.column as u64) + 1,
                    );
                } else {
                    fprintf(
                        stderr,
                        cstr!("Scanner error: %s at line %lu, column %lu\n"),
                        parser.problem,
                        (parser.problem_mark.line as u64) + 1,
                        (parser.problem_mark.column as u64) + 1,
                    );
                }
            }

            YAML_PARSER_ERROR => {
                if !parser.context.is_null() {
                    fprintf(
                        stderr,
                        cstr!("Parser error: %s at line %lu, column %lu\n%s at line %lu, column %lu\n"),
                        parser.context,
                        (parser.context_mark.line as u64) + 1,
                        (parser.context_mark.column as u64) + 1,
                        parser.problem,
                        (parser.problem_mark.line as u64) + 1,
                        (parser.problem_mark.column as u64) + 1,
                    );
                } else {
                    fprintf(
                        stderr,
                        cstr!("Parser error: %s at line %lu, column %lu\n"),
                        parser.problem,
                        (parser.problem_mark.line as u64) + 1,
                        (parser.problem_mark.column as u64) + 1,
                    );
                }
            }

            _ => {
                /* Couldn't happen. */
                fprintf(stderr, cstr!("Internal error\n"));
            }
        }

        yaml_event_delete(&mut input_event);
        yaml_document_delete(&mut output_document);
        yaml_parser_delete(&mut parser);
        yaml_emitter_delete(&mut emitter);

        return 1;
    }
}

fn main() {
    ctest::run(c_main)
}
