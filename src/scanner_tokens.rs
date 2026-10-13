//! Translation of `c_src/src/scanner.c` lines 1310-1937: the
//! `yaml_parser_fetch_*` token producers.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Initialize the scanner and produce the STREAM-START token.
 */

pub(crate) unsafe fn yaml_parser_fetch_stream_start(parser: *mut yaml_parser_t) -> c_int {
    let simple_key: yaml_simple_key_t = yaml_simple_key_t {
        possible: 0,
        required: 0,
        token_number: 0,
        mark: yaml_mark_t {
            index: 0,
            line: 0,
            column: 0,
        },
    };
    let mut token: yaml_token_t = core::mem::zeroed();

    /* Set the initial indentation. */

    (*parser).indent = -1;

    /* Initialize the simple key stack. */

    if !crate::PUSH!(parser, (*parser).simple_keys, simple_key) {
        return 0;
    }

    /* A simple key is allowed at the beginning of the stream. */

    (*parser).simple_key_allowed = 1;

    /* We have started. */

    (*parser).stream_start_produced = 1;

    /* Create the STREAM-START token and append it to the queue. */

    crate::STREAM_START_TOKEN_INIT!(
        token,
        (*parser).encoding,
        (*parser).mark,
        (*parser).mark
    );

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce the STREAM-END token and shut down the scanner.
 */

pub(crate) unsafe fn yaml_parser_fetch_stream_end(parser: *mut yaml_parser_t) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* Force new line. */

    if (*parser).mark.column != 0 {
        (*parser).mark.column = 0;
        (*parser).mark.line += 1;
    }

    /* Reset the indentation level. */

    if crate::scanner::yaml_parser_unroll_indent(parser, -1 as ptrdiff_t) == 0 {
        return 0;
    }

    /* Reset simple keys. */

    if crate::scanner::yaml_parser_remove_simple_key(parser) == 0 {
        return 0;
    }

    (*parser).simple_key_allowed = 0;

    /* Create the STREAM-END token and append it to the queue. */

    crate::STREAM_END_TOKEN_INIT!(token, (*parser).mark, (*parser).mark);

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce a VERSION-DIRECTIVE or TAG-DIRECTIVE token.
 */

pub(crate) unsafe fn yaml_parser_fetch_directive(parser: *mut yaml_parser_t) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* Reset the indentation level. */

    if crate::scanner::yaml_parser_unroll_indent(parser, -1 as ptrdiff_t) == 0 {
        return 0;
    }

    /* Reset simple keys. */

    if crate::scanner::yaml_parser_remove_simple_key(parser) == 0 {
        return 0;
    }

    (*parser).simple_key_allowed = 0;

    /* Create the YAML-DIRECTIVE or TAG-DIRECTIVE token. */

    if crate::scanner_scan::yaml_parser_scan_directive(parser, &raw mut token) == 0 {
        return 0;
    }

    /* Append the token to the queue. */

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        crate::api_objects::yaml_token_delete(&raw mut token);
        return 0;
    }

    1
}

/*
 * Produce the DOCUMENT-START or DOCUMENT-END token.
 */

pub(crate) unsafe fn yaml_parser_fetch_document_indicator(
    parser: *mut yaml_parser_t,
    type_: yaml_token_type_t,
) -> c_int {
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut token: yaml_token_t = core::mem::zeroed();

    /* Reset the indentation level. */

    if crate::scanner::yaml_parser_unroll_indent(parser, -1 as ptrdiff_t) == 0 {
        return 0;
    }

    /* Reset simple keys. */

    if crate::scanner::yaml_parser_remove_simple_key(parser) == 0 {
        return 0;
    }

    (*parser).simple_key_allowed = 0;

    /* Consume the token. */

    start_mark = (*parser).mark;

    crate::SKIP!(parser);
    crate::SKIP!(parser);
    crate::SKIP!(parser);

    end_mark = (*parser).mark;

    /* Create the DOCUMENT-START or DOCUMENT-END token. */

    crate::TOKEN_INIT!(token, type_, start_mark, end_mark);

    /* Append the token to the queue. */

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce the FLOW-SEQUENCE-START or FLOW-MAPPING-START token.
 */

pub(crate) unsafe fn yaml_parser_fetch_flow_collection_start(
    parser: *mut yaml_parser_t,
    type_: yaml_token_type_t,
) -> c_int {
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut token: yaml_token_t = core::mem::zeroed();

    /* The indicators '[' and '{' may start a simple key. */

    if crate::scanner::yaml_parser_save_simple_key(parser) == 0 {
        return 0;
    }

    /* Increase the flow level. */

    if crate::scanner::yaml_parser_increase_flow_level(parser) == 0 {
        return 0;
    }

    /* A simple key may follow the indicators '[' and '{'. */

    (*parser).simple_key_allowed = 1;

    /* Consume the token. */

    start_mark = (*parser).mark;
    crate::SKIP!(parser);
    end_mark = (*parser).mark;

    /* Create the FLOW-SEQUENCE-START of FLOW-MAPPING-START token. */

    crate::TOKEN_INIT!(token, type_, start_mark, end_mark);

    /* Append the token to the queue. */

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce the FLOW-SEQUENCE-END or FLOW-MAPPING-END token.
 */

pub(crate) unsafe fn yaml_parser_fetch_flow_collection_end(
    parser: *mut yaml_parser_t,
    type_: yaml_token_type_t,
) -> c_int {
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut token: yaml_token_t = core::mem::zeroed();

    /* Reset any potential simple key on the current flow level. */

    if crate::scanner::yaml_parser_remove_simple_key(parser) == 0 {
        return 0;
    }

    /* Decrease the flow level. */

    if crate::scanner::yaml_parser_decrease_flow_level(parser) == 0 {
        return 0;
    }

    /* No simple keys after the indicators ']' and '}'. */

    (*parser).simple_key_allowed = 0;

    /* Consume the token. */

    start_mark = (*parser).mark;
    crate::SKIP!(parser);
    end_mark = (*parser).mark;

    /* Create the FLOW-SEQUENCE-END of FLOW-MAPPING-END token. */

    crate::TOKEN_INIT!(token, type_, start_mark, end_mark);

    /* Append the token to the queue. */

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce the FLOW-ENTRY token.
 */

pub(crate) unsafe fn yaml_parser_fetch_flow_entry(parser: *mut yaml_parser_t) -> c_int {
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut token: yaml_token_t = core::mem::zeroed();

    /* Reset any potential simple keys on the current flow level. */

    if crate::scanner::yaml_parser_remove_simple_key(parser) == 0 {
        return 0;
    }

    /* Simple keys are allowed after ','. */

    (*parser).simple_key_allowed = 1;

    /* Consume the token. */

    start_mark = (*parser).mark;
    crate::SKIP!(parser);
    end_mark = (*parser).mark;

    /* Create the FLOW-ENTRY token and append it to the queue. */

    crate::TOKEN_INIT!(token, YAML_FLOW_ENTRY_TOKEN, start_mark, end_mark);

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce the BLOCK-ENTRY token.
 */

pub(crate) unsafe fn yaml_parser_fetch_block_entry(parser: *mut yaml_parser_t) -> c_int {
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut token: yaml_token_t = core::mem::zeroed();

    /* Check if the scanner is in the block context. */

    if (*parser).flow_level == 0 {
        /* Check if we are allowed to start a new entry. */

        if (*parser).simple_key_allowed == 0 {
            return crate::scanner::yaml_parser_set_scanner_error(
                parser,
                core::ptr::null::<c_char>(),
                (*parser).mark,
                b"block sequence entries are not allowed in this context\0".as_ptr()
                    as *const c_char,
            );
        }

        /* Add the BLOCK-SEQUENCE-START token if needed. */

        if crate::scanner::yaml_parser_roll_indent(
            parser,
            (*parser).mark.column as ptrdiff_t,
            -1 as ptrdiff_t,
            YAML_BLOCK_SEQUENCE_START_TOKEN,
            (*parser).mark,
        ) == 0
        {
            return 0;
        }
    } else {
        /*
         * It is an error for the '-' indicator to occur in the flow context,
         * but we let the Parser detect and report about it because the Parser
         * is able to point to the context.
         */
    }

    /* Reset any potential simple keys on the current flow level. */

    if crate::scanner::yaml_parser_remove_simple_key(parser) == 0 {
        return 0;
    }

    /* Simple keys are allowed after '-'. */

    (*parser).simple_key_allowed = 1;

    /* Consume the token. */

    start_mark = (*parser).mark;
    crate::SKIP!(parser);
    end_mark = (*parser).mark;

    /* Create the BLOCK-ENTRY token and append it to the queue. */

    crate::TOKEN_INIT!(token, YAML_BLOCK_ENTRY_TOKEN, start_mark, end_mark);

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce the KEY token.
 */

pub(crate) unsafe fn yaml_parser_fetch_key(parser: *mut yaml_parser_t) -> c_int {
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut token: yaml_token_t = core::mem::zeroed();

    /* In the block context, additional checks are required. */

    if (*parser).flow_level == 0 {
        /* Check if we are allowed to start a new key (not necessary simple). */

        if (*parser).simple_key_allowed == 0 {
            return crate::scanner::yaml_parser_set_scanner_error(
                parser,
                core::ptr::null::<c_char>(),
                (*parser).mark,
                b"mapping keys are not allowed in this context\0".as_ptr() as *const c_char,
            );
        }

        /* Add the BLOCK-MAPPING-START token if needed. */

        if crate::scanner::yaml_parser_roll_indent(
            parser,
            (*parser).mark.column as ptrdiff_t,
            -1 as ptrdiff_t,
            YAML_BLOCK_MAPPING_START_TOKEN,
            (*parser).mark,
        ) == 0
        {
            return 0;
        }
    }

    /* Reset any potential simple keys on the current flow level. */

    if crate::scanner::yaml_parser_remove_simple_key(parser) == 0 {
        return 0;
    }

    /* Simple keys are allowed after '?' in the block context. */

    (*parser).simple_key_allowed = ((*parser).flow_level == 0) as c_int;

    /* Consume the token. */

    start_mark = (*parser).mark;
    crate::SKIP!(parser);
    end_mark = (*parser).mark;

    /* Create the KEY token and append it to the queue. */

    crate::TOKEN_INIT!(token, YAML_KEY_TOKEN, start_mark, end_mark);

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce the VALUE token.
 */

pub(crate) unsafe fn yaml_parser_fetch_value(parser: *mut yaml_parser_t) -> c_int {
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut token: yaml_token_t = core::mem::zeroed();
    let simple_key: *mut yaml_simple_key_t = (*parser).simple_keys.top.sub(1);

    /* Have we found a simple key? */

    if (*simple_key).possible != 0 {
        /* Create the KEY token and insert it into the queue. */

        crate::TOKEN_INIT!(
            token,
            YAML_KEY_TOKEN,
            (*simple_key).mark,
            (*simple_key).mark
        );

        if !crate::QUEUE_INSERT!(
            parser,
            (*parser).tokens,
            (*simple_key).token_number.wrapping_sub((*parser).tokens_parsed),
            token
        ) {
            return 0;
        }

        /* In the block context, we may need to add the BLOCK-MAPPING-START token. */

        if crate::scanner::yaml_parser_roll_indent(
            parser,
            (*simple_key).mark.column as ptrdiff_t,
            (*simple_key).token_number as ptrdiff_t,
            YAML_BLOCK_MAPPING_START_TOKEN,
            (*simple_key).mark,
        ) == 0
        {
            return 0;
        }

        /* Remove the simple key. */

        (*simple_key).possible = 0;

        /* A simple key cannot follow another simple key. */

        (*parser).simple_key_allowed = 0;
    } else {
        /* The ':' indicator follows a complex key. */

        /* In the block context, extra checks are required. */

        if (*parser).flow_level == 0 {
            /* Check if we are allowed to start a complex value. */

            if (*parser).simple_key_allowed == 0 {
                return crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    core::ptr::null::<c_char>(),
                    (*parser).mark,
                    b"mapping values are not allowed in this context\0".as_ptr() as *const c_char,
                );
            }

            /* Add the BLOCK-MAPPING-START token if needed. */

            if crate::scanner::yaml_parser_roll_indent(
                parser,
                (*parser).mark.column as ptrdiff_t,
                -1 as ptrdiff_t,
                YAML_BLOCK_MAPPING_START_TOKEN,
                (*parser).mark,
            ) == 0
            {
                return 0;
            }
        }

        /* Simple keys after ':' are allowed in the block context. */

        (*parser).simple_key_allowed = ((*parser).flow_level == 0) as c_int;
    }

    /* Consume the token. */

    start_mark = (*parser).mark;
    crate::SKIP!(parser);
    end_mark = (*parser).mark;

    /* Create the VALUE token and append it to the queue. */

    crate::TOKEN_INIT!(token, YAML_VALUE_TOKEN, start_mark, end_mark);

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        return 0;
    }

    1
}

/*
 * Produce the ALIAS or ANCHOR token.
 */

pub(crate) unsafe fn yaml_parser_fetch_anchor(
    parser: *mut yaml_parser_t,
    type_: yaml_token_type_t,
) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* An anchor or an alias could be a simple key. */

    if crate::scanner::yaml_parser_save_simple_key(parser) == 0 {
        return 0;
    }

    /* A simple key cannot follow an anchor or an alias. */

    (*parser).simple_key_allowed = 0;

    /* Create the ALIAS or ANCHOR token and append it to the queue. */

    if crate::scanner_scan::yaml_parser_scan_anchor(parser, &raw mut token, type_) == 0 {
        return 0;
    }

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        crate::api_objects::yaml_token_delete(&raw mut token);
        return 0;
    }

    1
}

/*
 * Produce the TAG token.
 */

pub(crate) unsafe fn yaml_parser_fetch_tag(parser: *mut yaml_parser_t) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* A tag could be a simple key. */

    if crate::scanner::yaml_parser_save_simple_key(parser) == 0 {
        return 0;
    }

    /* A simple key cannot follow a tag. */

    (*parser).simple_key_allowed = 0;

    /* Create the TAG token and append it to the queue. */

    if crate::scanner_scan::yaml_parser_scan_tag(parser, &raw mut token) == 0 {
        return 0;
    }

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        crate::api_objects::yaml_token_delete(&raw mut token);
        return 0;
    }

    1
}

/*
 * Produce the SCALAR(...,literal) or SCALAR(...,folded) tokens.
 */

pub(crate) unsafe fn yaml_parser_fetch_block_scalar(
    parser: *mut yaml_parser_t,
    literal: c_int,
) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* Remove any potential simple keys. */

    if crate::scanner::yaml_parser_remove_simple_key(parser) == 0 {
        return 0;
    }

    /* A simple key may follow a block scalar. */

    (*parser).simple_key_allowed = 1;

    /* Create the SCALAR token and append it to the queue. */

    if crate::scanner_scalar::yaml_parser_scan_block_scalar(parser, &raw mut token, literal) == 0 {
        return 0;
    }

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        crate::api_objects::yaml_token_delete(&raw mut token);
        return 0;
    }

    1
}

/*
 * Produce the SCALAR(...,single-quoted) or SCALAR(...,double-quoted) tokens.
 */

pub(crate) unsafe fn yaml_parser_fetch_flow_scalar(
    parser: *mut yaml_parser_t,
    single: c_int,
) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* A plain scalar could be a simple key. */

    if crate::scanner::yaml_parser_save_simple_key(parser) == 0 {
        return 0;
    }

    /* A simple key cannot follow a flow scalar. */

    (*parser).simple_key_allowed = 0;

    /* Create the SCALAR token and append it to the queue. */

    if crate::scanner_scalar::yaml_parser_scan_flow_scalar(parser, &raw mut token, single) == 0 {
        return 0;
    }

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        crate::api_objects::yaml_token_delete(&raw mut token);
        return 0;
    }

    1
}

/*
 * Produce the SCALAR(...,plain) token.
 */

pub(crate) unsafe fn yaml_parser_fetch_plain_scalar(parser: *mut yaml_parser_t) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* A plain scalar could be a simple key. */

    if crate::scanner::yaml_parser_save_simple_key(parser) == 0 {
        return 0;
    }

    /* A simple key cannot follow a flow scalar. */

    (*parser).simple_key_allowed = 0;

    /* Create the SCALAR token and append it to the queue. */

    if crate::scanner_scalar::yaml_parser_scan_plain_scalar(parser, &raw mut token) == 0 {
        return 0;
    }

    if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
        crate::api_objects::yaml_token_delete(&raw mut token);
        return 0;
    }

    1
}
