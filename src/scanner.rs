//! Translation of `c_src/src/scanner.c` lines 744-1309: the public scanner
//! entry points, the token fetch loop, simple-key bookkeeping, flow level and
//! indentation management.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Get the next token.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_scan(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
) -> c_int {
    yaml_assert!(!parser.is_null()); /* Non-NULL parser object is expected. */
    yaml_assert!(!token.is_null()); /* Non-NULL token object is expected. */

    /* Erase the token object. */

    memset(
        token as *mut c_void,
        0,
        core::mem::size_of::<yaml_token_t>(),
    );

    /* No tokens after STREAM-END or error. */

    if (*parser).stream_end_produced != 0 || (*parser).error != 0 {
        return 1;
    }

    /* Ensure that the tokens queue contains enough tokens. */

    if (*parser).token_available == 0 {
        if yaml_parser_fetch_more_tokens(parser) == 0 {
            return 0;
        }
    }

    /* Fetch the next token from the queue. */

    *token = crate::DEQUEUE!(parser, (*parser).tokens);
    (*parser).token_available = 0;
    (*parser).tokens_parsed += 1;

    if (*token).type_ == YAML_STREAM_END_TOKEN {
        (*parser).stream_end_produced = 1;
    }

    1
}

/*
 * Set the scanner error and return 0.
 */

pub(crate) unsafe fn yaml_parser_set_scanner_error(
    parser: *mut yaml_parser_t,
    context: *const c_char,
    context_mark: yaml_mark_t,
    problem: *const c_char,
) -> c_int {
    (*parser).error = YAML_SCANNER_ERROR;
    (*parser).context = context;
    (*parser).context_mark = context_mark;
    (*parser).problem = problem;
    (*parser).problem_mark = (*parser).mark;

    0
}

/*
 * Ensure that the tokens queue contains at least one token which can be
 * returned to the Parser.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_fetch_more_tokens(parser: *mut yaml_parser_t) -> c_int {
    let mut need_more_tokens: c_int;

    /* While we need more tokens to fetch, do it. */

    loop {
        /*
         * Check if we really need to fetch more tokens.
         */

        need_more_tokens = 0;

        if (*parser).tokens.head == (*parser).tokens.tail {
            /* Queue is empty. */

            need_more_tokens = 1;
        } else {
            let mut simple_key: *mut yaml_simple_key_t;

            /* Check if any potential simple key may occupy the head position. */

            if yaml_parser_stale_simple_keys(parser) == 0 {
                return 0;
            }

            simple_key = (*parser).simple_keys.start;
            while simple_key != (*parser).simple_keys.top {
                if (*simple_key).possible != 0
                    && (*simple_key).token_number == (*parser).tokens_parsed
                {
                    need_more_tokens = 1;
                    break;
                }
                simple_key = simple_key.add(1);
            }
        }

        /* We are finished. */

        if need_more_tokens == 0 {
            break;
        }

        /* Fetch the next token. */

        if yaml_parser_fetch_next_token(parser) == 0 {
            return 0;
        }
    }

    (*parser).token_available = 1;

    1
}

/*
 * The dispatcher for token fetchers.
 */

pub(crate) unsafe fn yaml_parser_fetch_next_token(parser: *mut yaml_parser_t) -> c_int {
    /* Ensure that the buffer is initialized. */

    if !crate::CACHE!(parser, 1) {
        return 0;
    }

    /* Check if we just started scanning.  Fetch STREAM-START then. */

    if (*parser).stream_start_produced == 0 {
        return crate::scanner_tokens::yaml_parser_fetch_stream_start(parser);
    }

    /* Eat whitespaces and comments until we reach the next token. */

    if crate::scanner_scan::yaml_parser_scan_to_next_token(parser) == 0 {
        return 0;
    }

    /* Remove obsolete potential simple keys. */

    if yaml_parser_stale_simple_keys(parser) == 0 {
        return 0;
    }

    /* Check the indentation level against the current column. */

    if yaml_parser_unroll_indent(parser, (*parser).mark.column as ptrdiff_t) == 0 {
        return 0;
    }

    /*
     * Ensure that the buffer contains at least 4 characters.  4 is the length
     * of the longest indicators ('--- ' and '... ').
     */

    if !crate::CACHE!(parser, 4) {
        return 0;
    }

    /* Is it the end of the stream? */

    if crate::IS_Z!((*parser).buffer) {
        return crate::scanner_tokens::yaml_parser_fetch_stream_end(parser);
    }

    /* Is it a directive? */

    if (*parser).mark.column == 0 && crate::CHECK!((*parser).buffer, b'%') {
        return crate::scanner_tokens::yaml_parser_fetch_directive(parser);
    }

    /* Is it the document start indicator? */

    if (*parser).mark.column == 0
        && crate::CHECK_AT!((*parser).buffer, b'-', 0)
        && crate::CHECK_AT!((*parser).buffer, b'-', 1)
        && crate::CHECK_AT!((*parser).buffer, b'-', 2)
        && crate::IS_BLANKZ_AT!((*parser).buffer, 3)
    {
        return crate::scanner_tokens::yaml_parser_fetch_document_indicator(
            parser,
            YAML_DOCUMENT_START_TOKEN,
        );
    }

    /* Is it the document end indicator? */

    if (*parser).mark.column == 0
        && crate::CHECK_AT!((*parser).buffer, b'.', 0)
        && crate::CHECK_AT!((*parser).buffer, b'.', 1)
        && crate::CHECK_AT!((*parser).buffer, b'.', 2)
        && crate::IS_BLANKZ_AT!((*parser).buffer, 3)
    {
        return crate::scanner_tokens::yaml_parser_fetch_document_indicator(
            parser,
            YAML_DOCUMENT_END_TOKEN,
        );
    }

    /* Is it the flow sequence start indicator? */

    if crate::CHECK!((*parser).buffer, b'[') {
        return crate::scanner_tokens::yaml_parser_fetch_flow_collection_start(
            parser,
            YAML_FLOW_SEQUENCE_START_TOKEN,
        );
    }

    /* Is it the flow mapping start indicator? */

    if crate::CHECK!((*parser).buffer, b'{') {
        return crate::scanner_tokens::yaml_parser_fetch_flow_collection_start(
            parser,
            YAML_FLOW_MAPPING_START_TOKEN,
        );
    }

    /* Is it the flow sequence end indicator? */

    if crate::CHECK!((*parser).buffer, b']') {
        return crate::scanner_tokens::yaml_parser_fetch_flow_collection_end(
            parser,
            YAML_FLOW_SEQUENCE_END_TOKEN,
        );
    }

    /* Is it the flow mapping end indicator? */

    if crate::CHECK!((*parser).buffer, b'}') {
        return crate::scanner_tokens::yaml_parser_fetch_flow_collection_end(
            parser,
            YAML_FLOW_MAPPING_END_TOKEN,
        );
    }

    /* Is it the flow entry indicator? */

    if crate::CHECK!((*parser).buffer, b',') {
        return crate::scanner_tokens::yaml_parser_fetch_flow_entry(parser);
    }

    /* Is it the block entry indicator? */

    if crate::CHECK!((*parser).buffer, b'-') && crate::IS_BLANKZ_AT!((*parser).buffer, 1) {
        return crate::scanner_tokens::yaml_parser_fetch_block_entry(parser);
    }

    /* Is it the key indicator? */

    if crate::CHECK!((*parser).buffer, b'?')
        && ((*parser).flow_level != 0 || crate::IS_BLANKZ_AT!((*parser).buffer, 1))
    {
        return crate::scanner_tokens::yaml_parser_fetch_key(parser);
    }

    /* Is it the value indicator? */

    if crate::CHECK!((*parser).buffer, b':')
        && ((*parser).flow_level != 0 || crate::IS_BLANKZ_AT!((*parser).buffer, 1))
    {
        return crate::scanner_tokens::yaml_parser_fetch_value(parser);
    }

    /* Is it an alias? */

    if crate::CHECK!((*parser).buffer, b'*') {
        return crate::scanner_tokens::yaml_parser_fetch_anchor(parser, YAML_ALIAS_TOKEN);
    }

    /* Is it an anchor? */

    if crate::CHECK!((*parser).buffer, b'&') {
        return crate::scanner_tokens::yaml_parser_fetch_anchor(parser, YAML_ANCHOR_TOKEN);
    }

    /* Is it a tag? */

    if crate::CHECK!((*parser).buffer, b'!') {
        return crate::scanner_tokens::yaml_parser_fetch_tag(parser);
    }

    /* Is it a literal scalar? */

    if crate::CHECK!((*parser).buffer, b'|') && (*parser).flow_level == 0 {
        return crate::scanner_tokens::yaml_parser_fetch_block_scalar(parser, 1);
    }

    /* Is it a folded scalar? */

    if crate::CHECK!((*parser).buffer, b'>') && (*parser).flow_level == 0 {
        return crate::scanner_tokens::yaml_parser_fetch_block_scalar(parser, 0);
    }

    /* Is it a single-quoted scalar? */

    if crate::CHECK!((*parser).buffer, b'\'') {
        return crate::scanner_tokens::yaml_parser_fetch_flow_scalar(parser, 1);
    }

    /* Is it a double-quoted scalar? */

    if crate::CHECK!((*parser).buffer, b'"') {
        return crate::scanner_tokens::yaml_parser_fetch_flow_scalar(parser, 0);
    }

    /*
     * Is it a plain scalar?
     *
     * A plain scalar may start with any non-blank characters except
     *
     *      '-', '?', ':', ',', '[', ']', '{', '}',
     *      '#', '&', '*', '!', '|', '>', '\'', '\"',
     *      '%', '@', '`'.
     *
     * In the block context (and, for the '-' indicator, in the flow context
     * too), it may also start with the characters
     *
     *      '-', '?', ':'
     *
     * if it is followed by a non-space character.
     *
     * The last rule is more restrictive than the specification requires.
     */

    if !(crate::IS_BLANKZ!((*parser).buffer)
        || crate::CHECK!((*parser).buffer, b'-')
        || crate::CHECK!((*parser).buffer, b'?')
        || crate::CHECK!((*parser).buffer, b':')
        || crate::CHECK!((*parser).buffer, b',')
        || crate::CHECK!((*parser).buffer, b'[')
        || crate::CHECK!((*parser).buffer, b']')
        || crate::CHECK!((*parser).buffer, b'{')
        || crate::CHECK!((*parser).buffer, b'}')
        || crate::CHECK!((*parser).buffer, b'#')
        || crate::CHECK!((*parser).buffer, b'&')
        || crate::CHECK!((*parser).buffer, b'*')
        || crate::CHECK!((*parser).buffer, b'!')
        || crate::CHECK!((*parser).buffer, b'|')
        || crate::CHECK!((*parser).buffer, b'>')
        || crate::CHECK!((*parser).buffer, b'\'')
        || crate::CHECK!((*parser).buffer, b'"')
        || crate::CHECK!((*parser).buffer, b'%')
        || crate::CHECK!((*parser).buffer, b'@')
        || crate::CHECK!((*parser).buffer, b'`'))
        || (crate::CHECK!((*parser).buffer, b'-') && !crate::IS_BLANK_AT!((*parser).buffer, 1))
        || ((*parser).flow_level == 0
            && (crate::CHECK!((*parser).buffer, b'?') || crate::CHECK!((*parser).buffer, b':'))
            && !crate::IS_BLANKZ_AT!((*parser).buffer, 1))
    {
        return crate::scanner_tokens::yaml_parser_fetch_plain_scalar(parser);
    }

    /*
     * If we don't determine the token type so far, it is an error.
     */

    yaml_parser_set_scanner_error(
        parser,
        b"while scanning for the next token\0".as_ptr() as *const c_char,
        (*parser).mark,
        b"found character that cannot start any token\0".as_ptr() as *const c_char,
    )
}

/*
 * Check the list of potential simple keys and remove the positions that
 * cannot contain simple keys anymore.
 */

pub(crate) unsafe fn yaml_parser_stale_simple_keys(parser: *mut yaml_parser_t) -> c_int {
    let mut simple_key: *mut yaml_simple_key_t;

    /* Check for a potential simple key for each flow level. */

    simple_key = (*parser).simple_keys.start;
    while simple_key != (*parser).simple_keys.top {
        /*
         * The specification requires that a simple key
         *
         *  - is limited to a single line,
         *  - is shorter than 1024 characters.
         */

        if (*simple_key).possible != 0
            && ((*simple_key).mark.line < (*parser).mark.line
                || (*simple_key).mark.index.wrapping_add(1024) < (*parser).mark.index)
        {
            /* Check if the potential simple key to be removed is required. */

            if (*simple_key).required != 0 {
                return yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a simple key\0".as_ptr() as *const c_char,
                    (*simple_key).mark,
                    b"could not find expected ':'\0".as_ptr() as *const c_char,
                );
            }

            (*simple_key).possible = 0;
        }

        simple_key = simple_key.add(1);
    }

    1
}

/*
 * Check if a simple key may start at the current position and add it if
 * needed.
 */

pub(crate) unsafe fn yaml_parser_save_simple_key(parser: *mut yaml_parser_t) -> c_int {
    /*
     * A simple key is required at the current position if the scanner is in
     * the block context and the current column coincides with the indentation
     * level.
     */

    let required: c_int = ((*parser).flow_level == 0
        && (*parser).indent as ptrdiff_t == (*parser).mark.column as ptrdiff_t)
        as c_int;

    /*
     * If the current position may start a simple key, save it.
     */

    if (*parser).simple_key_allowed != 0 {
        let mut simple_key: yaml_simple_key_t = core::mem::zeroed();
        simple_key.possible = 1;
        simple_key.required = required;
        simple_key.token_number = (*parser).tokens_parsed.wrapping_add(
            (*parser).tokens.tail.offset_from((*parser).tokens.head) as size_t,
        );
        simple_key.mark = (*parser).mark;

        if yaml_parser_remove_simple_key(parser) == 0 {
            return 0;
        }

        *((*parser).simple_keys.top.sub(1)) = simple_key;
    }

    1
}

/*
 * Remove a potential simple key at the current flow level.
 */

pub(crate) unsafe fn yaml_parser_remove_simple_key(parser: *mut yaml_parser_t) -> c_int {
    let simple_key: *mut yaml_simple_key_t = (*parser).simple_keys.top.sub(1);

    if (*simple_key).possible != 0 {
        /* If the key is required, it is an error. */

        if (*simple_key).required != 0 {
            return yaml_parser_set_scanner_error(
                parser,
                b"while scanning a simple key\0".as_ptr() as *const c_char,
                (*simple_key).mark,
                b"could not find expected ':'\0".as_ptr() as *const c_char,
            );
        }
    }

    /* Remove the key from the stack. */

    (*simple_key).possible = 0;

    1
}

/*
 * Increase the flow level and resize the simple key list if needed.
 */

pub(crate) unsafe fn yaml_parser_increase_flow_level(parser: *mut yaml_parser_t) -> c_int {
    let empty_simple_key = yaml_simple_key_t {
        possible: 0,
        required: 0,
        token_number: 0,
        mark: yaml_mark_t {
            index: 0,
            line: 0,
            column: 0,
        },
    };

    /* Reset the simple key on the next level. */

    if !crate::PUSH!(parser, (*parser).simple_keys, empty_simple_key) {
        return 0;
    }

    /* Increase the flow level. */

    if (*parser).flow_level == INT_MAX {
        (*parser).error = YAML_MEMORY_ERROR;
        return 0;
    }

    if !crate::STACK_LIMIT!(
        parser,
        (*parser).indents,
        crate::private::MAX_NESTING_LEVEL - (*parser).flow_level
    ) {
        return yaml_parser_set_scanner_error(
            parser,
            b"while increasing flow level\0".as_ptr() as *const c_char,
            (*parser).mark,
            b"exceeded maximum nesting depth\0".as_ptr() as *const c_char,
        );
    }

    (*parser).flow_level += 1;

    1
}

/*
 * Decrease the flow level.
 */

pub(crate) unsafe fn yaml_parser_decrease_flow_level(parser: *mut yaml_parser_t) -> c_int {
    if (*parser).flow_level != 0 {
        (*parser).flow_level -= 1;
        let _ = crate::POP!(parser, (*parser).simple_keys);
    }

    1
}

/*
 * Push the current indentation level to the stack and set the new level
 * the current column is greater than the indentation level.  In this case,
 * append or insert the specified token into the token queue.
 *
 */

pub(crate) unsafe fn yaml_parser_roll_indent(
    parser: *mut yaml_parser_t,
    column: ptrdiff_t,
    number: ptrdiff_t,
    type_: yaml_token_type_t,
    mark: yaml_mark_t,
) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* In the flow context, do nothing. */

    if (*parser).flow_level != 0 {
        return 1;
    }

    if ((*parser).indent as ptrdiff_t) < column {
        /*
         * Push the current indentation level to the stack and set the new
         * indentation level.
         */

        if !crate::PUSH!(parser, (*parser).indents, (*parser).indent) {
            return 0;
        }

        if !crate::STACK_LIMIT!(
            parser,
            (*parser).indents,
            crate::private::MAX_NESTING_LEVEL - (*parser).flow_level
        ) {
            return yaml_parser_set_scanner_error(
                parser,
                b"while increasing block level\0".as_ptr() as *const c_char,
                (*parser).mark,
                b"exceeded maximum nesting depth\0".as_ptr() as *const c_char,
            );
        }

        if column > INT_MAX as ptrdiff_t {
            (*parser).error = YAML_MEMORY_ERROR;
            return 0;
        }

        (*parser).indent = column as c_int;

        /* Create a token and insert it into the queue. */

        crate::TOKEN_INIT!(token, type_, mark, mark);

        if number == -1 {
            if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
                return 0;
            }
        } else {
            if !crate::QUEUE_INSERT!(
                parser,
                (*parser).tokens,
                (number as size_t).wrapping_sub((*parser).tokens_parsed),
                token
            ) {
                return 0;
            }
        }
    }

    1
}

/*
 * Pop indentation levels from the indents stack until the current level
 * becomes less or equal to the column.  For each indentation level, append
 * the BLOCK-END token.
 */

pub(crate) unsafe fn yaml_parser_unroll_indent(
    parser: *mut yaml_parser_t,
    column: ptrdiff_t,
) -> c_int {
    let mut token: yaml_token_t = core::mem::zeroed();

    /* In the flow context, do nothing. */

    if (*parser).flow_level != 0 {
        return 1;
    }

    /* Loop through the indentation levels in the stack. */

    while ((*parser).indent as ptrdiff_t) > column {
        /* Create a token and append it to the queue. */

        crate::TOKEN_INIT!(
            token,
            YAML_BLOCK_END_TOKEN,
            (*parser).mark,
            (*parser).mark
        );

        if !crate::ENQUEUE!(parser, (*parser).tokens, token) {
            return 0;
        }

        /* Pop the indentation level. */

        (*parser).indent = crate::POP!(parser, (*parser).indents);
    }

    1
}
