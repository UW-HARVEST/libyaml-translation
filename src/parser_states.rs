//! Translation of `c_src/src/parser.c` lines 852-1398: block/flow mapping and
//! sequence states, empty scalars, and directive processing.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Parse the productions:
 * block_mapping        ::= BLOCK-MAPPING_START
 *                          *******************
 *                          ((KEY block_node_or_indentless_sequence?)?
 *                            *** *
 *                          (VALUE block_node_or_indentless_sequence?)?)*
 *
 *                          BLOCK-END
 *                          *********
 */

pub(crate) unsafe fn yaml_parser_parse_block_mapping_key(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    let mut token: *mut yaml_token_t;

    if first != 0 {
        token = crate::PEEK_TOKEN!(parser);
        if !crate::PUSH!(parser, (*parser).marks, (*token).start_mark) {
            return 0;
        }
        crate::SKIP_TOKEN!(parser);
    }

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ == YAML_KEY_TOKEN {
        let mark: yaml_mark_t = (*token).end_mark;
        crate::SKIP_TOKEN!(parser);
        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            return 0;
        }
        if (*token).type_ != YAML_KEY_TOKEN
            && (*token).type_ != YAML_VALUE_TOKEN
            && (*token).type_ != YAML_BLOCK_END_TOKEN
        {
            if !crate::PUSH!(
                parser,
                (*parser).states,
                YAML_PARSE_BLOCK_MAPPING_VALUE_STATE
            ) {
                return 0;
            }
            return crate::parser::yaml_parser_parse_node(parser, event, 1, 1);
        } else {
            (*parser).state = YAML_PARSE_BLOCK_MAPPING_VALUE_STATE;
            return yaml_parser_process_empty_scalar(parser, event, mark);
        }
    } else if (*token).type_ == YAML_BLOCK_END_TOKEN {
        (*parser).state = crate::POP!(parser, (*parser).states);
        let _ = crate::POP!(parser, (*parser).marks);
        crate::MAPPING_END_EVENT_INIT!((*event), (*token).start_mark, (*token).end_mark);
        crate::SKIP_TOKEN!(parser);
        return 1;
    } else {
        return crate::parser::yaml_parser_set_parser_error_context(
            parser,
            b"while parsing a block mapping\0".as_ptr() as *const c_char,
            crate::POP!(parser, (*parser).marks),
            b"did not find expected key\0".as_ptr() as *const c_char,
            (*token).start_mark,
        );
    }
}

/*
 * Parse the productions:
 * block_mapping        ::= BLOCK-MAPPING_START
 *
 *                          ((KEY block_node_or_indentless_sequence?)?
 *
 *                          (VALUE block_node_or_indentless_sequence?)?)*
 *                           ***** *
 *                          BLOCK-END
 *
 */

pub(crate) unsafe fn yaml_parser_parse_block_mapping_value(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let mut token: *mut yaml_token_t;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ == YAML_VALUE_TOKEN {
        let mark: yaml_mark_t = (*token).end_mark;
        crate::SKIP_TOKEN!(parser);
        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            return 0;
        }
        if (*token).type_ != YAML_KEY_TOKEN
            && (*token).type_ != YAML_VALUE_TOKEN
            && (*token).type_ != YAML_BLOCK_END_TOKEN
        {
            if !crate::PUSH!(
                parser,
                (*parser).states,
                YAML_PARSE_BLOCK_MAPPING_KEY_STATE
            ) {
                return 0;
            }
            return crate::parser::yaml_parser_parse_node(parser, event, 1, 1);
        } else {
            (*parser).state = YAML_PARSE_BLOCK_MAPPING_KEY_STATE;
            return yaml_parser_process_empty_scalar(parser, event, mark);
        }
    } else {
        (*parser).state = YAML_PARSE_BLOCK_MAPPING_KEY_STATE;
        return yaml_parser_process_empty_scalar(parser, event, (*token).start_mark);
    }
}

/*
 * Parse the productions:
 * flow_sequence        ::= FLOW-SEQUENCE-START
 *                          *******************
 *                          (flow_sequence_entry FLOW-ENTRY)*
 *                           *                   **********
 *                          flow_sequence_entry?
 *                          *
 *                          FLOW-SEQUENCE-END
 *                          *****************
 * flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
 *                          *
 */

pub(crate) unsafe fn yaml_parser_parse_flow_sequence_entry(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    let mut token: *mut yaml_token_t;

    if first != 0 {
        token = crate::PEEK_TOKEN!(parser);
        if !crate::PUSH!(parser, (*parser).marks, (*token).start_mark) {
            return 0;
        }
        crate::SKIP_TOKEN!(parser);
    }

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ != YAML_FLOW_SEQUENCE_END_TOKEN {
        if first == 0 {
            if (*token).type_ == YAML_FLOW_ENTRY_TOKEN {
                crate::SKIP_TOKEN!(parser);
                token = crate::PEEK_TOKEN!(parser);
                if token.is_null() {
                    return 0;
                }
            } else {
                return crate::parser::yaml_parser_set_parser_error_context(
                    parser,
                    b"while parsing a flow sequence\0".as_ptr() as *const c_char,
                    crate::POP!(parser, (*parser).marks),
                    b"did not find expected ',' or ']'\0".as_ptr() as *const c_char,
                    (*token).start_mark,
                );
            }
        }

        if (*token).type_ == YAML_KEY_TOKEN {
            (*parser).state = YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_KEY_STATE;
            crate::MAPPING_START_EVENT_INIT!(
                (*event),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                1,
                YAML_FLOW_MAPPING_STYLE,
                (*token).start_mark,
                (*token).end_mark
            );
            crate::SKIP_TOKEN!(parser);
            return 1;
        } else if (*token).type_ != YAML_FLOW_SEQUENCE_END_TOKEN {
            if !crate::PUSH!(
                parser,
                (*parser).states,
                YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE
            ) {
                return 0;
            }
            return crate::parser::yaml_parser_parse_node(parser, event, 0, 0);
        }
    }

    (*parser).state = crate::POP!(parser, (*parser).states);
    let _ = crate::POP!(parser, (*parser).marks);
    crate::SEQUENCE_END_EVENT_INIT!((*event), (*token).start_mark, (*token).end_mark);
    crate::SKIP_TOKEN!(parser);
    1
}

/*
 * Parse the productions:
 * flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
 *                                      *** *
 */

pub(crate) unsafe fn yaml_parser_parse_flow_sequence_entry_mapping_key(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token: *mut yaml_token_t;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ != YAML_VALUE_TOKEN
        && (*token).type_ != YAML_FLOW_ENTRY_TOKEN
        && (*token).type_ != YAML_FLOW_SEQUENCE_END_TOKEN
    {
        if !crate::PUSH!(
            parser,
            (*parser).states,
            YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE
        ) {
            return 0;
        }
        crate::parser::yaml_parser_parse_node(parser, event, 0, 0)
    } else if (*token).type_ == YAML_FLOW_SEQUENCE_END_TOKEN {
        let mark: yaml_mark_t = (*token).start_mark;
        (*parser).state = YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE;
        yaml_parser_process_empty_scalar(parser, event, mark)
    } else {
        let mark: yaml_mark_t = (*token).end_mark;
        crate::SKIP_TOKEN!(parser);
        (*parser).state = YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE;
        yaml_parser_process_empty_scalar(parser, event, mark)
    }
}

/*
 * Parse the productions:
 * flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
 *                                                      ***** *
 */

pub(crate) unsafe fn yaml_parser_parse_flow_sequence_entry_mapping_value(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let mut token: *mut yaml_token_t;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ == YAML_VALUE_TOKEN {
        crate::SKIP_TOKEN!(parser);
        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            return 0;
        }
        if (*token).type_ != YAML_FLOW_ENTRY_TOKEN
            && (*token).type_ != YAML_FLOW_SEQUENCE_END_TOKEN
        {
            if !crate::PUSH!(
                parser,
                (*parser).states,
                YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE
            ) {
                return 0;
            }
            return crate::parser::yaml_parser_parse_node(parser, event, 0, 0);
        }
    }
    (*parser).state = YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE;
    yaml_parser_process_empty_scalar(parser, event, (*token).start_mark)
}

/*
 * Parse the productions:
 * flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
 *                                                                      *
 */

pub(crate) unsafe fn yaml_parser_parse_flow_sequence_entry_mapping_end(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token: *mut yaml_token_t;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    (*parser).state = YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE;

    crate::MAPPING_END_EVENT_INIT!((*event), (*token).start_mark, (*token).start_mark);
    1
}

/*
 * Parse the productions:
 * flow_mapping         ::= FLOW-MAPPING-START
 *                          ******************
 *                          (flow_mapping_entry FLOW-ENTRY)*
 *                           *                  **********
 *                          flow_mapping_entry?
 *                          ******************
 *                          FLOW-MAPPING-END
 *                          ****************
 * flow_mapping_entry   ::= flow_node | KEY flow_node? (VALUE flow_node?)?
 *                          *           *** *
 */

pub(crate) unsafe fn yaml_parser_parse_flow_mapping_key(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    let mut token: *mut yaml_token_t;

    if first != 0 {
        token = crate::PEEK_TOKEN!(parser);
        if !crate::PUSH!(parser, (*parser).marks, (*token).start_mark) {
            return 0;
        }
        crate::SKIP_TOKEN!(parser);
    }

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ != YAML_FLOW_MAPPING_END_TOKEN {
        if first == 0 {
            if (*token).type_ == YAML_FLOW_ENTRY_TOKEN {
                crate::SKIP_TOKEN!(parser);
                token = crate::PEEK_TOKEN!(parser);
                if token.is_null() {
                    return 0;
                }
            } else {
                return crate::parser::yaml_parser_set_parser_error_context(
                    parser,
                    b"while parsing a flow mapping\0".as_ptr() as *const c_char,
                    crate::POP!(parser, (*parser).marks),
                    b"did not find expected ',' or '}'\0".as_ptr() as *const c_char,
                    (*token).start_mark,
                );
            }
        }

        if (*token).type_ == YAML_KEY_TOKEN {
            crate::SKIP_TOKEN!(parser);
            token = crate::PEEK_TOKEN!(parser);
            if token.is_null() {
                return 0;
            }
            if (*token).type_ != YAML_VALUE_TOKEN
                && (*token).type_ != YAML_FLOW_ENTRY_TOKEN
                && (*token).type_ != YAML_FLOW_MAPPING_END_TOKEN
            {
                if !crate::PUSH!(
                    parser,
                    (*parser).states,
                    YAML_PARSE_FLOW_MAPPING_VALUE_STATE
                ) {
                    return 0;
                }
                return crate::parser::yaml_parser_parse_node(parser, event, 0, 0);
            } else {
                (*parser).state = YAML_PARSE_FLOW_MAPPING_VALUE_STATE;
                return yaml_parser_process_empty_scalar(parser, event, (*token).start_mark);
            }
        } else if (*token).type_ != YAML_FLOW_MAPPING_END_TOKEN {
            if !crate::PUSH!(
                parser,
                (*parser).states,
                YAML_PARSE_FLOW_MAPPING_EMPTY_VALUE_STATE
            ) {
                return 0;
            }
            return crate::parser::yaml_parser_parse_node(parser, event, 0, 0);
        }
    }

    (*parser).state = crate::POP!(parser, (*parser).states);
    let _ = crate::POP!(parser, (*parser).marks);
    crate::MAPPING_END_EVENT_INIT!((*event), (*token).start_mark, (*token).end_mark);
    crate::SKIP_TOKEN!(parser);
    1
}

/*
 * Parse the productions:
 * flow_mapping_entry   ::= flow_node | KEY flow_node? (VALUE flow_node?)?
 *                                   *                  ***** *
 */

pub(crate) unsafe fn yaml_parser_parse_flow_mapping_value(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    empty: c_int,
) -> c_int {
    let mut token: *mut yaml_token_t;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if empty != 0 {
        (*parser).state = YAML_PARSE_FLOW_MAPPING_KEY_STATE;
        return yaml_parser_process_empty_scalar(parser, event, (*token).start_mark);
    }

    if (*token).type_ == YAML_VALUE_TOKEN {
        crate::SKIP_TOKEN!(parser);
        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            return 0;
        }
        if (*token).type_ != YAML_FLOW_ENTRY_TOKEN
            && (*token).type_ != YAML_FLOW_MAPPING_END_TOKEN
        {
            if !crate::PUSH!(
                parser,
                (*parser).states,
                YAML_PARSE_FLOW_MAPPING_KEY_STATE
            ) {
                return 0;
            }
            return crate::parser::yaml_parser_parse_node(parser, event, 0, 0);
        }
    }

    (*parser).state = YAML_PARSE_FLOW_MAPPING_KEY_STATE;
    yaml_parser_process_empty_scalar(parser, event, (*token).start_mark)
}

/*
 * Generate an empty scalar event.
 */

pub(crate) unsafe fn yaml_parser_process_empty_scalar(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    mark: yaml_mark_t,
) -> c_int {
    let value: *mut yaml_char_t;

    value = crate::memory::yaml_malloc(1) as *mut yaml_char_t;
    if value.is_null() {
        (*parser).error = YAML_MEMORY_ERROR;
        return 0;
    }
    *value.add(0) = b'\0';

    crate::SCALAR_EVENT_INIT!(
        (*event),
        core::ptr::null_mut(),
        core::ptr::null_mut(),
        value,
        0,
        1,
        0,
        YAML_PLAIN_SCALAR_STYLE,
        mark,
        mark
    );

    1
}

/*
 * Parse directives.
 */

pub(crate) unsafe fn yaml_parser_process_directives(
    parser: *mut yaml_parser_t,
    version_directive_ref: *mut *mut yaml_version_directive_t,
    tag_directives_start_ref: *mut *mut yaml_tag_directive_t,
    tag_directives_end_ref: *mut *mut yaml_tag_directive_t,
) -> c_int {
    let default_tag_directives: [yaml_tag_directive_t; 3] = [
        yaml_tag_directive_t {
            handle: b"!\0".as_ptr() as *mut yaml_char_t,
            prefix: b"!\0".as_ptr() as *mut yaml_char_t,
        },
        yaml_tag_directive_t {
            handle: b"!!\0".as_ptr() as *mut yaml_char_t,
            prefix: b"tag:yaml.org,2002:\0".as_ptr() as *mut yaml_char_t,
        },
        yaml_tag_directive_t {
            handle: core::ptr::null_mut(),
            prefix: core::ptr::null_mut(),
        },
    ];
    let mut default_tag_directive: *const yaml_tag_directive_t;
    let mut version_directive: *mut yaml_version_directive_t = core::ptr::null_mut();
    let mut tag_directives: yaml_stack_t<yaml_tag_directive_t> = yaml_stack_t::zeroed();
    let mut token: *mut yaml_token_t;

    'error: {
        if !crate::STACK_INIT!(parser, tag_directives) {
            break 'error;
        }

        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            break 'error;
        }

        while (*token).type_ == YAML_VERSION_DIRECTIVE_TOKEN
            || (*token).type_ == YAML_TAG_DIRECTIVE_TOKEN
        {
            if (*token).type_ == YAML_VERSION_DIRECTIVE_TOKEN {
                if !version_directive.is_null() {
                    crate::parser::yaml_parser_set_parser_error(
                        parser,
                        b"found duplicate %YAML directive\0".as_ptr() as *const c_char,
                        (*token).start_mark,
                    );
                    break 'error;
                }
                if (*token).data.version_directive.major != 1
                    || ((*token).data.version_directive.minor != 1
                        && (*token).data.version_directive.minor != 2)
                {
                    crate::parser::yaml_parser_set_parser_error(
                        parser,
                        b"found incompatible YAML document\0".as_ptr() as *const c_char,
                        (*token).start_mark,
                    );
                    break 'error;
                }
                version_directive = crate::memory::yaml_malloc(core::mem::size_of::<
                    yaml_version_directive_t,
                >()) as *mut yaml_version_directive_t;
                if version_directive.is_null() {
                    (*parser).error = YAML_MEMORY_ERROR;
                    break 'error;
                }
                (*version_directive).major = (*token).data.version_directive.major;
                (*version_directive).minor = (*token).data.version_directive.minor;
            } else if (*token).type_ == YAML_TAG_DIRECTIVE_TOKEN {
                let mut value: yaml_tag_directive_t = yaml_tag_directive_t {
                    handle: core::ptr::null_mut(),
                    prefix: core::ptr::null_mut(),
                };
                value.handle = (*token).data.tag_directive.handle;
                value.prefix = (*token).data.tag_directive.prefix;

                if yaml_parser_append_tag_directive(parser, value, 0, (*token).start_mark) == 0 {
                    break 'error;
                }
                if !crate::PUSH!(parser, tag_directives, value) {
                    break 'error;
                }
            }

            crate::SKIP_TOKEN!(parser);
            token = crate::PEEK_TOKEN!(parser);
            if token.is_null() {
                break 'error;
            }
        }

        default_tag_directive = default_tag_directives.as_ptr();
        while !(*default_tag_directive).handle.is_null() {
            if yaml_parser_append_tag_directive(
                parser,
                *default_tag_directive,
                1,
                (*token).start_mark,
            ) == 0
            {
                break 'error;
            }
            default_tag_directive = default_tag_directive.add(1);
        }

        if !version_directive_ref.is_null() {
            *version_directive_ref = version_directive;
        }
        if !tag_directives_start_ref.is_null() {
            if crate::STACK_EMPTY!(parser, tag_directives) {
                *tag_directives_end_ref = core::ptr::null_mut();
                *tag_directives_start_ref = *tag_directives_end_ref;
                crate::STACK_DEL!(parser, tag_directives);
            } else {
                *tag_directives_start_ref = tag_directives.start;
                *tag_directives_end_ref = tag_directives.top;
            }
        } else {
            crate::STACK_DEL!(parser, tag_directives);
        }

        if version_directive_ref.is_null() {
            crate::memory::yaml_free(version_directive as *mut c_void);
        }
        return 1;
    }

    /* error: */
    crate::memory::yaml_free(version_directive as *mut c_void);
    while !crate::STACK_EMPTY!(parser, tag_directives) {
        let tag_directive: yaml_tag_directive_t = crate::POP!(parser, tag_directives);
        crate::memory::yaml_free(tag_directive.handle as *mut c_void);
        crate::memory::yaml_free(tag_directive.prefix as *mut c_void);
    }
    crate::STACK_DEL!(parser, tag_directives);
    0
}

/*
 * Append a tag directive to the directives stack.
 */

pub(crate) unsafe fn yaml_parser_append_tag_directive(
    parser: *mut yaml_parser_t,
    value: yaml_tag_directive_t,
    allow_duplicates: c_int,
    mark: yaml_mark_t,
) -> c_int {
    let mut tag_directive: *mut yaml_tag_directive_t;
    let mut copy: yaml_tag_directive_t = yaml_tag_directive_t {
        handle: core::ptr::null_mut(),
        prefix: core::ptr::null_mut(),
    };

    'error: {
        tag_directive = (*parser).tag_directives.start;
        while tag_directive != (*parser).tag_directives.top {
            if strcmp(
                value.handle as *const c_char,
                (*tag_directive).handle as *const c_char,
            ) == 0
            {
                if allow_duplicates != 0 {
                    return 1;
                }
                return crate::parser::yaml_parser_set_parser_error(
                    parser,
                    b"found duplicate %TAG directive\0".as_ptr() as *const c_char,
                    mark,
                );
            }
            tag_directive = tag_directive.add(1);
        }

        copy.handle = crate::memory::yaml_strdup(value.handle);
        copy.prefix = crate::memory::yaml_strdup(value.prefix);
        if copy.handle.is_null() || copy.prefix.is_null() {
            (*parser).error = YAML_MEMORY_ERROR;
            break 'error;
        }

        if !crate::PUSH!(parser, (*parser).tag_directives, copy) {
            break 'error;
        }

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(copy.handle as *mut c_void);
    crate::memory::yaml_free(copy.prefix as *mut c_void);
    0
}
