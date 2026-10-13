//! Translation of `c_src/src/parser.c` lines 1-851: the parser entry point,
//! error helpers, the state machine, and the document/node/block-sequence
//! states.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * The parser implements the following grammar:
 *
 * stream               ::= STREAM-START implicit_document? explicit_document* STREAM-END
 * implicit_document    ::= block_node DOCUMENT-END*
 * explicit_document    ::= DIRECTIVE* DOCUMENT-START block_node? DOCUMENT-END*
 * block_node_or_indentless_sequence    ::=
 *                          ALIAS
 *                          | properties (block_content | indentless_block_sequence)?
 *                          | block_content
 *                          | indentless_block_sequence
 * block_node           ::= ALIAS
 *                          | properties block_content?
 *                          | block_content
 * flow_node            ::= ALIAS
 *                          | properties flow_content?
 *                          | flow_content
 * properties           ::= TAG ANCHOR? | ANCHOR TAG?
 * block_content        ::= block_collection | flow_collection | SCALAR
 * flow_content         ::= flow_collection | SCALAR
 * block_collection     ::= block_sequence | block_mapping
 * flow_collection      ::= flow_sequence | flow_mapping
 * block_sequence       ::= BLOCK-SEQUENCE-START (BLOCK-ENTRY block_node?)* BLOCK-END
 * indentless_sequence  ::= (BLOCK-ENTRY block_node?)+
 * block_mapping        ::= BLOCK-MAPPING_START
 *                          ((KEY block_node_or_indentless_sequence?)?
 *                          (VALUE block_node_or_indentless_sequence?)?)*
 *                          BLOCK-END
 * flow_sequence        ::= FLOW-SEQUENCE-START
 *                          (flow_sequence_entry FLOW-ENTRY)*
 *                          flow_sequence_entry?
 *                          FLOW-SEQUENCE-END
 * flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
 * flow_mapping         ::= FLOW-MAPPING-START
 *                          (flow_mapping_entry FLOW-ENTRY)*
 *                          flow_mapping_entry?
 *                          FLOW-MAPPING-END
 * flow_mapping_entry   ::= flow_node | KEY flow_node? (VALUE flow_node?)?
 */

/*
 * `PEEK_TOKEN(parser)` and `SKIP_TOKEN(parser)` live in `crate::private`.
 * `int MAX_NESTING_LEVEL = 1000;` lives there too (it is a real exported
 * symbol, so it must be defined exactly once for the whole crate).
 */

/*
 * Recursion limit controls.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_get_max_nest_level() -> c_int {
    crate::private::MAX_NESTING_LEVEL
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_set_max_nest_level(max: c_int) {
    crate::private::MAX_NESTING_LEVEL = max;
}

/*
 * Get the next event.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_parse(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    yaml_assert!(!parser.is_null()); /* Non-NULL parser object is expected. */
    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */

    /* Erase the event object. */

    memset(
        event as *mut c_void,
        0,
        core::mem::size_of::<yaml_event_t>(),
    );

    /* No events after the end of the stream or error. */

    if (*parser).stream_end_produced != 0
        || (*parser).error != YAML_NO_ERROR
        || (*parser).state == YAML_PARSE_END_STATE
    {
        return 1;
    }

    /* Generate the next event. */

    yaml_parser_state_machine(parser, event)
}

/*
 * Set parser error.
 */

pub(crate) unsafe fn yaml_parser_set_parser_error(
    parser: *mut yaml_parser_t,
    problem: *const c_char,
    problem_mark: yaml_mark_t,
) -> c_int {
    (*parser).error = YAML_PARSER_ERROR;
    (*parser).problem = problem;
    (*parser).problem_mark = problem_mark;

    0
}

pub(crate) unsafe fn yaml_parser_set_parser_error_context(
    parser: *mut yaml_parser_t,
    context: *const c_char,
    context_mark: yaml_mark_t,
    problem: *const c_char,
    problem_mark: yaml_mark_t,
) -> c_int {
    (*parser).error = YAML_PARSER_ERROR;
    (*parser).context = context;
    (*parser).context_mark = context_mark;
    (*parser).problem = problem;
    (*parser).problem_mark = problem_mark;

    0
}

/*
 * State dispatcher.
 */

pub(crate) unsafe fn yaml_parser_state_machine(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    match (*parser).state {
        YAML_PARSE_STREAM_START_STATE => {
            return yaml_parser_parse_stream_start(parser, event);
        }

        YAML_PARSE_IMPLICIT_DOCUMENT_START_STATE => {
            return yaml_parser_parse_document_start(parser, event, 1);
        }

        YAML_PARSE_DOCUMENT_START_STATE => {
            return yaml_parser_parse_document_start(parser, event, 0);
        }

        YAML_PARSE_DOCUMENT_CONTENT_STATE => {
            return yaml_parser_parse_document_content(parser, event);
        }

        YAML_PARSE_DOCUMENT_END_STATE => {
            return yaml_parser_parse_document_end(parser, event);
        }

        YAML_PARSE_BLOCK_NODE_STATE => {
            return yaml_parser_parse_node(parser, event, 1, 0);
        }

        YAML_PARSE_BLOCK_NODE_OR_INDENTLESS_SEQUENCE_STATE => {
            return yaml_parser_parse_node(parser, event, 1, 1);
        }

        YAML_PARSE_FLOW_NODE_STATE => {
            return yaml_parser_parse_node(parser, event, 0, 0);
        }

        YAML_PARSE_BLOCK_SEQUENCE_FIRST_ENTRY_STATE => {
            return yaml_parser_parse_block_sequence_entry(parser, event, 1);
        }

        YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE => {
            return yaml_parser_parse_block_sequence_entry(parser, event, 0);
        }

        YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE => {
            return yaml_parser_parse_indentless_sequence_entry(parser, event);
        }

        YAML_PARSE_BLOCK_MAPPING_FIRST_KEY_STATE => {
            return crate::parser_states::yaml_parser_parse_block_mapping_key(parser, event, 1);
        }

        YAML_PARSE_BLOCK_MAPPING_KEY_STATE => {
            return crate::parser_states::yaml_parser_parse_block_mapping_key(parser, event, 0);
        }

        YAML_PARSE_BLOCK_MAPPING_VALUE_STATE => {
            return crate::parser_states::yaml_parser_parse_block_mapping_value(parser, event);
        }

        YAML_PARSE_FLOW_SEQUENCE_FIRST_ENTRY_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_sequence_entry(parser, event, 1);
        }

        YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_sequence_entry(parser, event, 0);
        }

        YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_KEY_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_sequence_entry_mapping_key(
                parser, event,
            );
        }

        YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_sequence_entry_mapping_value(
                parser, event,
            );
        }

        YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_sequence_entry_mapping_end(
                parser, event,
            );
        }

        YAML_PARSE_FLOW_MAPPING_FIRST_KEY_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_mapping_key(parser, event, 1);
        }

        YAML_PARSE_FLOW_MAPPING_KEY_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_mapping_key(parser, event, 0);
        }

        YAML_PARSE_FLOW_MAPPING_VALUE_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_mapping_value(parser, event, 0);
        }

        YAML_PARSE_FLOW_MAPPING_EMPTY_VALUE_STATE => {
            return crate::parser_states::yaml_parser_parse_flow_mapping_value(parser, event, 1);
        }

        _ => {
            /* The C source really does say `assert(1)` here, not `assert(0)`,
             * so the default branch never aborts — it just falls through to
             * `return 0`.  Reproduced verbatim. */
            yaml_assert!(true); /* Invalid state. */
        }
    }

    0
}

/*
 * Parse the production:
 * stream   ::= STREAM-START implicit_document? explicit_document* STREAM-END
 *              ************
 */

pub(crate) unsafe fn yaml_parser_parse_stream_start(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token: *mut yaml_token_t;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ != YAML_STREAM_START_TOKEN {
        return yaml_parser_set_parser_error(
            parser,
            b"did not find expected <stream-start>\0".as_ptr() as *const c_char,
            (*token).start_mark,
        );
    }

    (*parser).state = YAML_PARSE_IMPLICIT_DOCUMENT_START_STATE;
    crate::STREAM_START_EVENT_INIT!(
        (*event),
        (*token).data.stream_start.encoding,
        (*token).start_mark,
        (*token).start_mark
    );
    crate::SKIP_TOKEN!(parser);

    1
}

/*
 * Parse the productions:
 * implicit_document    ::= block_node DOCUMENT-END*
 *                          *
 * explicit_document    ::= DIRECTIVE* DOCUMENT-START block_node? DOCUMENT-END*
 *                          *************************
 */

pub(crate) unsafe fn yaml_parser_parse_document_start(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    implicit: c_int,
) -> c_int {
    let mut token: *mut yaml_token_t;
    let mut version_directive: *mut yaml_version_directive_t = core::ptr::null_mut();
    let mut tag_directives = yaml_tag_directives_range_t {
        start: core::ptr::null_mut(),
        end: core::ptr::null_mut(),
    };

    'error: {
        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            return 0;
        }

        /* Parse extra document end indicators. */

        if implicit == 0 {
            while (*token).type_ == YAML_DOCUMENT_END_TOKEN {
                crate::SKIP_TOKEN!(parser);
                token = crate::PEEK_TOKEN!(parser);
                if token.is_null() {
                    return 0;
                }
            }
        }

        /* Parse an implicit document. */

        if implicit != 0
            && (*token).type_ != YAML_VERSION_DIRECTIVE_TOKEN
            && (*token).type_ != YAML_TAG_DIRECTIVE_TOKEN
            && (*token).type_ != YAML_DOCUMENT_START_TOKEN
            && (*token).type_ != YAML_STREAM_END_TOKEN
        {
            if crate::parser_states::yaml_parser_process_directives(
                parser,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            ) == 0
            {
                return 0;
            }
            if !crate::PUSH!(parser, (*parser).states, YAML_PARSE_DOCUMENT_END_STATE) {
                return 0;
            }
            (*parser).state = YAML_PARSE_BLOCK_NODE_STATE;
            crate::DOCUMENT_START_EVENT_INIT!(
                (*event),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                1,
                (*token).start_mark,
                (*token).start_mark
            );
            return 1;
        }
        /* Parse an explicit document. */
        else if (*token).type_ != YAML_STREAM_END_TOKEN {
            let start_mark: yaml_mark_t;
            let end_mark: yaml_mark_t;
            start_mark = (*token).start_mark;
            if crate::parser_states::yaml_parser_process_directives(
                parser,
                &raw mut version_directive,
                &raw mut tag_directives.start,
                &raw mut tag_directives.end,
            ) == 0
            {
                return 0;
            }
            token = crate::PEEK_TOKEN!(parser);
            if token.is_null() {
                break 'error;
            }
            if (*token).type_ != YAML_DOCUMENT_START_TOKEN {
                yaml_parser_set_parser_error(
                    parser,
                    b"did not find expected <document start>\0".as_ptr() as *const c_char,
                    (*token).start_mark,
                );
                break 'error;
            }
            if !crate::PUSH!(parser, (*parser).states, YAML_PARSE_DOCUMENT_END_STATE) {
                break 'error;
            }
            (*parser).state = YAML_PARSE_DOCUMENT_CONTENT_STATE;
            end_mark = (*token).end_mark;
            crate::DOCUMENT_START_EVENT_INIT!(
                (*event),
                version_directive,
                tag_directives.start,
                tag_directives.end,
                0,
                start_mark,
                end_mark
            );
            crate::SKIP_TOKEN!(parser);
            version_directive = core::ptr::null_mut();
            tag_directives.end = core::ptr::null_mut();
            tag_directives.start = tag_directives.end;
            return 1;
        }
        /* Parse the stream end. */
        else {
            (*parser).state = YAML_PARSE_END_STATE;
            crate::STREAM_END_EVENT_INIT!(
                (*event),
                (*token).start_mark,
                (*token).end_mark
            );
            crate::SKIP_TOKEN!(parser);
            return 1;
        }
    }

    /* error: */
    crate::memory::yaml_free(version_directive as *mut c_void);
    while tag_directives.start != tag_directives.end {
        crate::memory::yaml_free((*tag_directives.end.offset(-1)).handle as *mut c_void);
        crate::memory::yaml_free((*tag_directives.end.offset(-1)).prefix as *mut c_void);
        tag_directives.end = tag_directives.end.offset(-1);
    }
    crate::memory::yaml_free(tag_directives.start as *mut c_void);
    0
}

/*
 * Parse the productions:
 * explicit_document    ::= DIRECTIVE* DOCUMENT-START block_node? DOCUMENT-END*
 *                                                    ***********
 */

pub(crate) unsafe fn yaml_parser_parse_document_content(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token: *mut yaml_token_t;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ == YAML_VERSION_DIRECTIVE_TOKEN
        || (*token).type_ == YAML_TAG_DIRECTIVE_TOKEN
        || (*token).type_ == YAML_DOCUMENT_START_TOKEN
        || (*token).type_ == YAML_DOCUMENT_END_TOKEN
        || (*token).type_ == YAML_STREAM_END_TOKEN
    {
        (*parser).state = crate::POP!(parser, (*parser).states);
        crate::parser_states::yaml_parser_process_empty_scalar(
            parser,
            event,
            (*token).start_mark,
        )
    } else {
        yaml_parser_parse_node(parser, event, 1, 0)
    }
}

/*
 * Parse the productions:
 * implicit_document    ::= block_node DOCUMENT-END*
 *                                     *************
 * explicit_document    ::= DIRECTIVE* DOCUMENT-START block_node? DOCUMENT-END*
 *                                                                *************
 */

pub(crate) unsafe fn yaml_parser_parse_document_end(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token: *mut yaml_token_t;
    let start_mark: yaml_mark_t;
    let mut end_mark: yaml_mark_t;
    let mut implicit: c_int = 1;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    end_mark = (*token).start_mark;
    start_mark = end_mark;

    if (*token).type_ == YAML_DOCUMENT_END_TOKEN {
        end_mark = (*token).end_mark;
        crate::SKIP_TOKEN!(parser);
        implicit = 0;
    }

    while !crate::STACK_EMPTY!(parser, (*parser).tag_directives) {
        let tag_directive: yaml_tag_directive_t = crate::POP!(parser, (*parser).tag_directives);
        crate::memory::yaml_free(tag_directive.handle as *mut c_void);
        crate::memory::yaml_free(tag_directive.prefix as *mut c_void);
    }

    (*parser).state = YAML_PARSE_DOCUMENT_START_STATE;
    crate::DOCUMENT_END_EVENT_INIT!((*event), implicit, start_mark, end_mark);

    1
}

/*
 * Parse the productions:
 * block_node_or_indentless_sequence    ::=
 *                          ALIAS
 *                          *****
 *                          | properties (block_content | indentless_block_sequence)?
 *                            **********  *
 *                          | block_content | indentless_block_sequence
 *                            *
 * block_node           ::= ALIAS
 *                          *****
 *                          | properties block_content?
 *                            ********** *
 *                          | block_content
 *                            *
 * flow_node            ::= ALIAS
 *                          *****
 *                          | properties flow_content?
 *                            ********** *
 *                          | flow_content
 *                            *
 * properties           ::= TAG ANCHOR? | ANCHOR TAG?
 *                          *************************
 * block_content        ::= block_collection | flow_collection | SCALAR
 *                                                               ******
 * flow_content         ::= flow_collection | SCALAR
 *                                            ******
 */

pub(crate) unsafe fn yaml_parser_parse_node(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    block: c_int,
    indentless_sequence: c_int,
) -> c_int {
    let mut token: *mut yaml_token_t;
    let mut anchor: *mut yaml_char_t = core::ptr::null_mut();
    let mut tag_handle: *mut yaml_char_t = core::ptr::null_mut();
    let mut tag_suffix: *mut yaml_char_t = core::ptr::null_mut();
    let mut tag: *mut yaml_char_t = core::ptr::null_mut();
    /* `start_mark`, `end_mark` and `tag_mark` are uninitialised in C; they are
     * always written before being read, so zeroing them changes nothing. */
    let mut start_mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut end_mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut tag_mark: yaml_mark_t = yaml_mark_t::zeroed();
    let implicit: c_int;

    'error: {
        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            return 0;
        }

        if (*token).type_ == YAML_ALIAS_TOKEN {
            (*parser).state = crate::POP!(parser, (*parser).states);
            crate::ALIAS_EVENT_INIT!(
                (*event),
                (*token).data.alias.value,
                (*token).start_mark,
                (*token).end_mark
            );
            crate::SKIP_TOKEN!(parser);
            return 1;
        } else {
            end_mark = (*token).start_mark;
            start_mark = end_mark;

            if (*token).type_ == YAML_ANCHOR_TOKEN {
                anchor = (*token).data.anchor.value;
                start_mark = (*token).start_mark;
                end_mark = (*token).end_mark;
                crate::SKIP_TOKEN!(parser);
                token = crate::PEEK_TOKEN!(parser);
                if token.is_null() {
                    break 'error;
                }
                if (*token).type_ == YAML_TAG_TOKEN {
                    tag_handle = (*token).data.tag.handle;
                    tag_suffix = (*token).data.tag.suffix;
                    tag_mark = (*token).start_mark;
                    end_mark = (*token).end_mark;
                    crate::SKIP_TOKEN!(parser);
                    token = crate::PEEK_TOKEN!(parser);
                    if token.is_null() {
                        break 'error;
                    }
                }
            } else if (*token).type_ == YAML_TAG_TOKEN {
                tag_handle = (*token).data.tag.handle;
                tag_suffix = (*token).data.tag.suffix;
                tag_mark = (*token).start_mark;
                start_mark = tag_mark;
                end_mark = (*token).end_mark;
                crate::SKIP_TOKEN!(parser);
                token = crate::PEEK_TOKEN!(parser);
                if token.is_null() {
                    break 'error;
                }
                if (*token).type_ == YAML_ANCHOR_TOKEN {
                    anchor = (*token).data.anchor.value;
                    end_mark = (*token).end_mark;
                    crate::SKIP_TOKEN!(parser);
                    token = crate::PEEK_TOKEN!(parser);
                    if token.is_null() {
                        break 'error;
                    }
                }
            }

            if !tag_handle.is_null() {
                if *tag_handle == 0 {
                    tag = tag_suffix;
                    crate::memory::yaml_free(tag_handle as *mut c_void);
                    tag_suffix = core::ptr::null_mut();
                    tag_handle = tag_suffix;
                } else {
                    let mut tag_directive: *mut yaml_tag_directive_t;
                    tag_directive = (*parser).tag_directives.start;
                    while tag_directive != (*parser).tag_directives.top {
                        if strcmp(
                            (*tag_directive).handle as *const c_char,
                            tag_handle as *const c_char,
                        ) == 0
                        {
                            let prefix_len: size_t =
                                strlen((*tag_directive).prefix as *const c_char);
                            let suffix_len: size_t = strlen(tag_suffix as *const c_char);
                            tag = crate::memory::yaml_malloc(prefix_len + suffix_len + 1)
                                as *mut yaml_char_t;
                            if tag.is_null() {
                                (*parser).error = YAML_MEMORY_ERROR;
                                break 'error;
                            }
                            memcpy(
                                tag as *mut c_void,
                                (*tag_directive).prefix as *const c_void,
                                prefix_len,
                            );
                            memcpy(
                                tag.add(prefix_len) as *mut c_void,
                                tag_suffix as *const c_void,
                                suffix_len,
                            );
                            *tag.add(prefix_len + suffix_len) = b'\0';
                            crate::memory::yaml_free(tag_handle as *mut c_void);
                            crate::memory::yaml_free(tag_suffix as *mut c_void);
                            tag_suffix = core::ptr::null_mut();
                            tag_handle = tag_suffix;
                            break;
                        }
                        tag_directive = tag_directive.add(1);
                    }
                    if tag.is_null() {
                        yaml_parser_set_parser_error_context(
                            parser,
                            b"while parsing a node\0".as_ptr() as *const c_char,
                            start_mark,
                            b"found undefined tag handle\0".as_ptr() as *const c_char,
                            tag_mark,
                        );
                        break 'error;
                    }
                }
            }

            implicit = (tag.is_null() || *tag == 0) as c_int;
            if indentless_sequence != 0 && (*token).type_ == YAML_BLOCK_ENTRY_TOKEN {
                end_mark = (*token).end_mark;
                (*parser).state = YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE;
                crate::SEQUENCE_START_EVENT_INIT!(
                    (*event),
                    anchor,
                    tag,
                    implicit,
                    YAML_BLOCK_SEQUENCE_STYLE,
                    start_mark,
                    end_mark
                );
                return 1;
            } else {
                if (*token).type_ == YAML_SCALAR_TOKEN {
                    let mut plain_implicit: c_int = 0;
                    let mut quoted_implicit: c_int = 0;
                    end_mark = (*token).end_mark;
                    if ((*token).data.scalar.style == YAML_PLAIN_SCALAR_STYLE && tag.is_null())
                        || (!tag.is_null()
                            && strcmp(
                                tag as *const c_char,
                                b"!\0".as_ptr() as *const c_char,
                            ) == 0)
                    {
                        plain_implicit = 1;
                    } else if tag.is_null() {
                        quoted_implicit = 1;
                    }
                    (*parser).state = crate::POP!(parser, (*parser).states);
                    crate::SCALAR_EVENT_INIT!(
                        (*event),
                        anchor,
                        tag,
                        (*token).data.scalar.value,
                        (*token).data.scalar.length,
                        plain_implicit,
                        quoted_implicit,
                        (*token).data.scalar.style,
                        start_mark,
                        end_mark
                    );
                    crate::SKIP_TOKEN!(parser);
                    return 1;
                } else if (*token).type_ == YAML_FLOW_SEQUENCE_START_TOKEN {
                    end_mark = (*token).end_mark;
                    (*parser).state = YAML_PARSE_FLOW_SEQUENCE_FIRST_ENTRY_STATE;
                    crate::SEQUENCE_START_EVENT_INIT!(
                        (*event),
                        anchor,
                        tag,
                        implicit,
                        YAML_FLOW_SEQUENCE_STYLE,
                        start_mark,
                        end_mark
                    );
                    return 1;
                } else if (*token).type_ == YAML_FLOW_MAPPING_START_TOKEN {
                    end_mark = (*token).end_mark;
                    (*parser).state = YAML_PARSE_FLOW_MAPPING_FIRST_KEY_STATE;
                    crate::MAPPING_START_EVENT_INIT!(
                        (*event),
                        anchor,
                        tag,
                        implicit,
                        YAML_FLOW_MAPPING_STYLE,
                        start_mark,
                        end_mark
                    );
                    return 1;
                } else if block != 0 && (*token).type_ == YAML_BLOCK_SEQUENCE_START_TOKEN {
                    end_mark = (*token).end_mark;
                    (*parser).state = YAML_PARSE_BLOCK_SEQUENCE_FIRST_ENTRY_STATE;
                    crate::SEQUENCE_START_EVENT_INIT!(
                        (*event),
                        anchor,
                        tag,
                        implicit,
                        YAML_BLOCK_SEQUENCE_STYLE,
                        start_mark,
                        end_mark
                    );
                    return 1;
                } else if block != 0 && (*token).type_ == YAML_BLOCK_MAPPING_START_TOKEN {
                    end_mark = (*token).end_mark;
                    (*parser).state = YAML_PARSE_BLOCK_MAPPING_FIRST_KEY_STATE;
                    crate::MAPPING_START_EVENT_INIT!(
                        (*event),
                        anchor,
                        tag,
                        implicit,
                        YAML_BLOCK_MAPPING_STYLE,
                        start_mark,
                        end_mark
                    );
                    return 1;
                } else if !anchor.is_null() || !tag.is_null() {
                    let value: *mut yaml_char_t =
                        crate::memory::yaml_malloc(1) as *mut yaml_char_t;
                    if value.is_null() {
                        (*parser).error = YAML_MEMORY_ERROR;
                        break 'error;
                    }
                    *value.add(0) = b'\0';
                    (*parser).state = crate::POP!(parser, (*parser).states);
                    crate::SCALAR_EVENT_INIT!(
                        (*event),
                        anchor,
                        tag,
                        value,
                        0,
                        implicit,
                        0,
                        YAML_PLAIN_SCALAR_STYLE,
                        start_mark,
                        end_mark
                    );
                    return 1;
                } else {
                    yaml_parser_set_parser_error_context(
                        parser,
                        if block != 0 {
                            b"while parsing a block node\0".as_ptr() as *const c_char
                        } else {
                            b"while parsing a flow node\0".as_ptr() as *const c_char
                        },
                        start_mark,
                        b"did not find expected node content\0".as_ptr() as *const c_char,
                        (*token).start_mark,
                    );
                    break 'error;
                }
            }
        }
    }

    /* error: */
    crate::memory::yaml_free(anchor as *mut c_void);
    crate::memory::yaml_free(tag_handle as *mut c_void);
    crate::memory::yaml_free(tag_suffix as *mut c_void);
    crate::memory::yaml_free(tag as *mut c_void);

    0
}

/*
 * Parse the productions:
 * block_sequence ::= BLOCK-SEQUENCE-START (BLOCK-ENTRY block_node?)* BLOCK-END
 *                    ********************  *********** *             *********
 */

pub(crate) unsafe fn yaml_parser_parse_block_sequence_entry(
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

    if (*token).type_ == YAML_BLOCK_ENTRY_TOKEN {
        let mark: yaml_mark_t = (*token).end_mark;
        crate::SKIP_TOKEN!(parser);
        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            return 0;
        }
        if (*token).type_ != YAML_BLOCK_ENTRY_TOKEN && (*token).type_ != YAML_BLOCK_END_TOKEN {
            if !crate::PUSH!(
                parser,
                (*parser).states,
                YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE
            ) {
                return 0;
            }
            yaml_parser_parse_node(parser, event, 1, 0)
        } else {
            (*parser).state = YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE;
            crate::parser_states::yaml_parser_process_empty_scalar(parser, event, mark)
        }
    } else if (*token).type_ == YAML_BLOCK_END_TOKEN {
        (*parser).state = crate::POP!(parser, (*parser).states);
        let _ = crate::POP!(parser, (*parser).marks);
        crate::SEQUENCE_END_EVENT_INIT!((*event), (*token).start_mark, (*token).end_mark);
        crate::SKIP_TOKEN!(parser);
        1
    } else {
        yaml_parser_set_parser_error_context(
            parser,
            b"while parsing a block collection\0".as_ptr() as *const c_char,
            crate::POP!(parser, (*parser).marks),
            b"did not find expected '-' indicator\0".as_ptr() as *const c_char,
            (*token).start_mark,
        )
    }
}

/*
 * Parse the productions:
 * indentless_sequence  ::= (BLOCK-ENTRY block_node?)+
 *                           *********** *
 */

pub(crate) unsafe fn yaml_parser_parse_indentless_sequence_entry(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let mut token: *mut yaml_token_t;

    token = crate::PEEK_TOKEN!(parser);
    if token.is_null() {
        return 0;
    }

    if (*token).type_ == YAML_BLOCK_ENTRY_TOKEN {
        let mark: yaml_mark_t = (*token).end_mark;
        crate::SKIP_TOKEN!(parser);
        token = crate::PEEK_TOKEN!(parser);
        if token.is_null() {
            return 0;
        }
        if (*token).type_ != YAML_BLOCK_ENTRY_TOKEN
            && (*token).type_ != YAML_KEY_TOKEN
            && (*token).type_ != YAML_VALUE_TOKEN
            && (*token).type_ != YAML_BLOCK_END_TOKEN
        {
            if !crate::PUSH!(
                parser,
                (*parser).states,
                YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE
            ) {
                return 0;
            }
            yaml_parser_parse_node(parser, event, 1, 0)
        } else {
            (*parser).state = YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE;
            crate::parser_states::yaml_parser_process_empty_scalar(parser, event, mark)
        }
    } else {
        (*parser).state = crate::POP!(parser, (*parser).states);
        crate::SEQUENCE_END_EVENT_INIT!((*event), (*token).start_mark, (*token).start_mark);
        1
    }
}
