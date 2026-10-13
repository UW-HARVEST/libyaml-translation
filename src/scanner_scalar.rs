//! Translation of `c_src/src/scanner.c` lines 2759-3616: the block, flow and
//! plain scalar scanners.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Scan a block scalar.
 */

pub(crate) unsafe fn yaml_parser_scan_block_scalar(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
    literal: c_int,
) -> c_int {
    let mut start_mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut end_mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut leading_break: yaml_string_t = crate::NULL_STRING!();
    let mut trailing_breaks: yaml_string_t = crate::NULL_STRING!();
    let mut chomping: c_int = 0;
    let mut increment: c_int = 0;
    let mut indent: c_int = 0;
    let mut leading_blank: c_int = 0;
    let mut trailing_blank: c_int = 0;

    'error: {
        if !crate::STRING_INIT!(parser, string, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }
        if !crate::STRING_INIT!(parser, leading_break, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }
        if !crate::STRING_INIT!(parser, trailing_breaks, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }

        /* Eat the indicator '|' or '>'. */

        start_mark = (*parser).mark;

        crate::SKIP!(parser);

        /* Scan the additional block scalar indicators. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        /* Check for a chomping indicator. */

        if crate::CHECK!((*parser).buffer, b'+') || crate::CHECK!((*parser).buffer, b'-') {
            /* Set the chomping method and eat the indicator. */

            chomping = if crate::CHECK!((*parser).buffer, b'+') {
                1
            } else {
                -1
            };

            crate::SKIP!(parser);

            /* Check for an indentation indicator. */

            if !crate::CACHE!(parser, 1) {
                break 'error;
            }

            if crate::IS_DIGIT!((*parser).buffer) {
                /* Check that the indentation is greater than 0. */

                if crate::CHECK!((*parser).buffer, b'0') {
                    crate::scanner::yaml_parser_set_scanner_error(
                        parser,
                        b"while scanning a block scalar\0".as_ptr() as *const c_char,
                        start_mark,
                        b"found an indentation indicator equal to 0\0".as_ptr() as *const c_char,
                    );
                    break 'error;
                }

                /* Get the indentation level and eat the indicator. */

                increment = crate::AS_DIGIT!((*parser).buffer);

                crate::SKIP!(parser);
            }
        }
        /* Do the same as above, but in the opposite order. */
        else if crate::IS_DIGIT!((*parser).buffer) {
            if crate::CHECK!((*parser).buffer, b'0') {
                crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a block scalar\0".as_ptr() as *const c_char,
                    start_mark,
                    b"found an indentation indicator equal to 0\0".as_ptr() as *const c_char,
                );
                break 'error;
            }

            increment = crate::AS_DIGIT!((*parser).buffer);

            crate::SKIP!(parser);

            if !crate::CACHE!(parser, 1) {
                break 'error;
            }

            if crate::CHECK!((*parser).buffer, b'+') || crate::CHECK!((*parser).buffer, b'-') {
                chomping = if crate::CHECK!((*parser).buffer, b'+') {
                    1
                } else {
                    -1
                };

                crate::SKIP!(parser);
            }
        }

        /* Eat whitespaces and comments to the end of the line. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        while crate::IS_BLANK!((*parser).buffer) {
            crate::SKIP!(parser);
            if !crate::CACHE!(parser, 1) {
                break 'error;
            }
        }

        if crate::CHECK!((*parser).buffer, b'#') {
            while !crate::IS_BREAKZ!((*parser).buffer) {
                crate::SKIP!(parser);
                if !crate::CACHE!(parser, 1) {
                    break 'error;
                }
            }
        }

        /* Check if we are at the end of the line. */

        if !crate::IS_BREAKZ!((*parser).buffer) {
            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                b"while scanning a block scalar\0".as_ptr() as *const c_char,
                start_mark,
                b"did not find expected comment or line break\0".as_ptr() as *const c_char,
            );
            break 'error;
        }

        /* Eat a line break. */

        if crate::IS_BREAK!((*parser).buffer) {
            if !crate::CACHE!(parser, 2) {
                break 'error;
            }
            crate::SKIP_LINE!(parser);
        }

        end_mark = (*parser).mark;

        /* Set the indentation level if it was specified. */

        if increment != 0 {
            indent = if (*parser).indent >= 0 {
                (*parser).indent + increment
            } else {
                increment
            };
        }

        /* Scan the leading line breaks and determine the indentation level if needed. */

        if yaml_parser_scan_block_scalar_breaks(
            parser,
            &raw mut indent,
            &raw mut trailing_breaks,
            start_mark,
            &raw mut end_mark,
        ) == 0
        {
            break 'error;
        }

        /* Scan the block scalar content. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        while (*parser).mark.column as c_int == indent && !(crate::IS_Z!((*parser).buffer)) {
            /*
             * We are at the beginning of a non-empty line.
             */

            /* Is it a trailing whitespace? */

            trailing_blank = crate::IS_BLANK!((*parser).buffer) as c_int;

            /* Check if we need to fold the leading line break. */

            if literal == 0
                && (*leading_break.start == b'\n')
                && leading_blank == 0
                && trailing_blank == 0
            {
                /* Do we need to join the lines by space? */

                if *trailing_breaks.start == b'\0' {
                    if !crate::STRING_EXTEND!(parser, string) {
                        break 'error;
                    }
                    *string.pointer = b' ';
                    string.pointer = string.pointer.add(1);
                }

                crate::CLEAR!(parser, leading_break);
            } else {
                if !crate::JOIN!(parser, string, leading_break) {
                    break 'error;
                }
                crate::CLEAR!(parser, leading_break);
            }

            /* Append the remaining line breaks. */

            if !crate::JOIN!(parser, string, trailing_breaks) {
                break 'error;
            }
            crate::CLEAR!(parser, trailing_breaks);

            /* Is it a leading whitespace? */

            leading_blank = crate::IS_BLANK!((*parser).buffer) as c_int;

            /* Consume the current line. */

            while !crate::IS_BREAKZ!((*parser).buffer) {
                if !crate::READ!(parser, string) {
                    break 'error;
                }
                if !crate::CACHE!(parser, 1) {
                    break 'error;
                }
            }

            /* Consume the line break. */

            if !crate::CACHE!(parser, 2) {
                break 'error;
            }

            if !crate::READ_LINE!(parser, leading_break) {
                break 'error;
            }

            /* Eat the following indentation spaces and line breaks. */

            if yaml_parser_scan_block_scalar_breaks(
                parser,
                &raw mut indent,
                &raw mut trailing_breaks,
                start_mark,
                &raw mut end_mark,
            ) == 0
            {
                break 'error;
            }
        }

        /* Chomp the tail. */

        if chomping != -1 {
            if !crate::JOIN!(parser, string, leading_break) {
                break 'error;
            }
        }
        if chomping == 1 {
            if !crate::JOIN!(parser, string, trailing_breaks) {
                break 'error;
            }
        }

        /* Create a token. */

        crate::SCALAR_TOKEN_INIT!(
            *token,
            string.start,
            string.pointer.offset_from(string.start) as size_t,
            if literal != 0 {
                YAML_LITERAL_SCALAR_STYLE
            } else {
                YAML_FOLDED_SCALAR_STYLE
            },
            start_mark,
            end_mark
        );

        crate::STRING_DEL!(parser, leading_break);
        crate::STRING_DEL!(parser, trailing_breaks);

        return 1;
    }

    /* error: */

    crate::STRING_DEL!(parser, string);
    crate::STRING_DEL!(parser, leading_break);
    crate::STRING_DEL!(parser, trailing_breaks);

    0
}

/*
 * Scan indentation spaces and line breaks for a block scalar.  Determine the
 * indentation level if needed.
 */

pub(crate) unsafe fn yaml_parser_scan_block_scalar_breaks(
    parser: *mut yaml_parser_t,
    indent: *mut c_int,
    breaks: *mut yaml_string_t,
    start_mark: yaml_mark_t,
    end_mark: *mut yaml_mark_t,
) -> c_int {
    let mut max_indent: c_int = 0;

    *end_mark = (*parser).mark;

    /* Eat the indentation spaces and line breaks. */

    loop {
        /* Eat the indentation spaces. */

        if !crate::CACHE!(parser, 1) {
            return 0;
        }

        while (*indent == 0 || ((*parser).mark.column as c_int) < *indent)
            && crate::IS_SPACE!((*parser).buffer)
        {
            crate::SKIP!(parser);
            if !crate::CACHE!(parser, 1) {
                return 0;
            }
        }

        if (*parser).mark.column as c_int > max_indent {
            max_indent = (*parser).mark.column as c_int;
        }

        /* Check for a tab character messing the indentation. */

        if (*indent == 0 || ((*parser).mark.column as c_int) < *indent)
            && crate::IS_TAB!((*parser).buffer)
        {
            return crate::scanner::yaml_parser_set_scanner_error(
                parser,
                b"while scanning a block scalar\0".as_ptr() as *const c_char,
                start_mark,
                b"found a tab character where an indentation space is expected\0".as_ptr()
                    as *const c_char,
            );
        }

        /* Have we found a non-empty line? */

        if !crate::IS_BREAK!((*parser).buffer) {
            break;
        }

        /* Consume the line break. */

        if !crate::CACHE!(parser, 2) {
            return 0;
        }
        if !crate::READ_LINE!(parser, (*breaks)) {
            return 0;
        }
        *end_mark = (*parser).mark;
    }

    /* Determine the indentation level if needed. */

    if *indent == 0 {
        *indent = max_indent;
        if *indent < (*parser).indent + 1 {
            *indent = (*parser).indent + 1;
        }
        if *indent < 1 {
            *indent = 1;
        }
    }

    1
}

/*
 * Scan a quoted scalar.
 */

pub(crate) unsafe fn yaml_parser_scan_flow_scalar(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
    single: c_int,
) -> c_int {
    let mut start_mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut end_mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut leading_break: yaml_string_t = crate::NULL_STRING!();
    let mut trailing_breaks: yaml_string_t = crate::NULL_STRING!();
    let mut whitespaces: yaml_string_t = crate::NULL_STRING!();
    let mut leading_blanks: c_int;

    'error: {
        if !crate::STRING_INIT!(parser, string, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }
        if !crate::STRING_INIT!(parser, leading_break, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }
        if !crate::STRING_INIT!(parser, trailing_breaks, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }
        if !crate::STRING_INIT!(parser, whitespaces, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }

        /* Eat the left quote. */

        start_mark = (*parser).mark;

        crate::SKIP!(parser);

        /* Consume the content of the quoted scalar. */

        loop {
            /* Check that there are no document indicators at the beginning of the line. */

            if !crate::CACHE!(parser, 4) {
                break 'error;
            }

            if (*parser).mark.column == 0
                && ((crate::CHECK_AT!((*parser).buffer, b'-', 0)
                    && crate::CHECK_AT!((*parser).buffer, b'-', 1)
                    && crate::CHECK_AT!((*parser).buffer, b'-', 2))
                    || (crate::CHECK_AT!((*parser).buffer, b'.', 0)
                        && crate::CHECK_AT!((*parser).buffer, b'.', 1)
                        && crate::CHECK_AT!((*parser).buffer, b'.', 2)))
                && crate::IS_BLANKZ_AT!((*parser).buffer, 3)
            {
                crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a quoted scalar\0".as_ptr() as *const c_char,
                    start_mark,
                    b"found unexpected document indicator\0".as_ptr() as *const c_char,
                );
                break 'error;
            }

            /* Check for EOF. */

            if crate::IS_Z!((*parser).buffer) {
                crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a quoted scalar\0".as_ptr() as *const c_char,
                    start_mark,
                    b"found unexpected end of stream\0".as_ptr() as *const c_char,
                );
                break 'error;
            }

            /* Consume non-blank characters. */

            if !crate::CACHE!(parser, 2) {
                break 'error;
            }

            leading_blanks = 0;

            while !crate::IS_BLANKZ!((*parser).buffer) {
                /* Check for an escaped single quote. */

                if single != 0
                    && crate::CHECK_AT!((*parser).buffer, b'\'', 0)
                    && crate::CHECK_AT!((*parser).buffer, b'\'', 1)
                {
                    if !crate::STRING_EXTEND!(parser, string) {
                        break 'error;
                    }
                    *string.pointer = b'\'';
                    string.pointer = string.pointer.add(1);
                    crate::SKIP!(parser);
                    crate::SKIP!(parser);
                }
                /* Check for the right quote. */
                else if crate::CHECK!(
                    (*parser).buffer,
                    (if single != 0 { b'\'' } else { b'"' })
                ) {
                    break;
                }
                /* Check for an escaped line break. */
                else if single == 0
                    && crate::CHECK!((*parser).buffer, b'\\')
                    && crate::IS_BREAK_AT!((*parser).buffer, 1)
                {
                    if !crate::CACHE!(parser, 3) {
                        break 'error;
                    }
                    crate::SKIP!(parser);
                    crate::SKIP_LINE!(parser);
                    leading_blanks = 1;
                    break;
                }
                /* Check for an escape sequence. */
                else if single == 0 && crate::CHECK!((*parser).buffer, b'\\') {
                    let mut code_length: size_t = 0;

                    if !crate::STRING_EXTEND!(parser, string) {
                        break 'error;
                    }

                    /* Check the escape character. */

                    match *(*parser).buffer.pointer.add(1) {
                        b'0' => {
                            *string.pointer = b'\0';
                            string.pointer = string.pointer.add(1);
                        }

                        b'a' => {
                            *string.pointer = 0x07;
                            string.pointer = string.pointer.add(1);
                        }

                        b'b' => {
                            *string.pointer = 0x08;
                            string.pointer = string.pointer.add(1);
                        }

                        b't' | b'\t' => {
                            *string.pointer = 0x09;
                            string.pointer = string.pointer.add(1);
                        }

                        b'n' => {
                            *string.pointer = 0x0A;
                            string.pointer = string.pointer.add(1);
                        }

                        b'v' => {
                            *string.pointer = 0x0B;
                            string.pointer = string.pointer.add(1);
                        }

                        b'f' => {
                            *string.pointer = 0x0C;
                            string.pointer = string.pointer.add(1);
                        }

                        b'r' => {
                            *string.pointer = 0x0D;
                            string.pointer = string.pointer.add(1);
                        }

                        b'e' => {
                            *string.pointer = 0x1B;
                            string.pointer = string.pointer.add(1);
                        }

                        b' ' => {
                            *string.pointer = 0x20;
                            string.pointer = string.pointer.add(1);
                        }

                        b'"' => {
                            *string.pointer = b'"';
                            string.pointer = string.pointer.add(1);
                        }

                        b'/' => {
                            *string.pointer = b'/';
                            string.pointer = string.pointer.add(1);
                        }

                        b'\\' => {
                            *string.pointer = b'\\';
                            string.pointer = string.pointer.add(1);
                        }

                        /* NEL (#x85) */
                        b'N' => {
                            *string.pointer = 0xC2;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = 0x85;
                            string.pointer = string.pointer.add(1);
                        }

                        /* #xA0 */
                        b'_' => {
                            *string.pointer = 0xC2;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = 0xA0;
                            string.pointer = string.pointer.add(1);
                        }

                        /* LS (#x2028) */
                        b'L' => {
                            *string.pointer = 0xE2;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = 0x80;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = 0xA8;
                            string.pointer = string.pointer.add(1);
                        }

                        /* PS (#x2029) */
                        b'P' => {
                            *string.pointer = 0xE2;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = 0x80;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = 0xA9;
                            string.pointer = string.pointer.add(1);
                        }

                        b'x' => {
                            code_length = 2;
                        }

                        b'u' => {
                            code_length = 4;
                        }

                        b'U' => {
                            code_length = 8;
                        }

                        _ => {
                            crate::scanner::yaml_parser_set_scanner_error(
                                parser,
                                b"while parsing a quoted scalar\0".as_ptr() as *const c_char,
                                start_mark,
                                b"found unknown escape character\0".as_ptr() as *const c_char,
                            );
                            break 'error;
                        }
                    }

                    crate::SKIP!(parser);
                    crate::SKIP!(parser);

                    /* Consume an arbitrary escape code. */

                    if code_length != 0 {
                        let mut value: c_uint = 0;
                        let mut k: size_t;

                        /* Scan the character value. */

                        if !crate::CACHE!(parser, code_length) {
                            break 'error;
                        }

                        k = 0;
                        while k < code_length {
                            if !crate::IS_HEX_AT!((*parser).buffer, k) {
                                crate::scanner::yaml_parser_set_scanner_error(
                                    parser,
                                    b"while parsing a quoted scalar\0".as_ptr() as *const c_char,
                                    start_mark,
                                    b"did not find expected hexdecimal number\0".as_ptr()
                                        as *const c_char,
                                );
                                break 'error;
                            }
                            value = (value << 4) + crate::AS_HEX_AT!((*parser).buffer, k) as c_uint;
                            k += 1;
                        }

                        /* Check the value and write the character. */

                        if (value >= 0xD800 && value <= 0xDFFF) || value > 0x10FFFF {
                            crate::scanner::yaml_parser_set_scanner_error(
                                parser,
                                b"while parsing a quoted scalar\0".as_ptr() as *const c_char,
                                start_mark,
                                b"found invalid Unicode character escape code\0".as_ptr()
                                    as *const c_char,
                            );
                            break 'error;
                        }

                        if value <= 0x7F {
                            *string.pointer = value as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                        } else if value <= 0x7FF {
                            *string.pointer = (0xC0 + (value >> 6)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = (0x80 + (value & 0x3F)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                        } else if value <= 0xFFFF {
                            *string.pointer = (0xE0 + (value >> 12)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = (0x80 + ((value >> 6) & 0x3F)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = (0x80 + (value & 0x3F)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                        } else {
                            *string.pointer = (0xF0 + (value >> 18)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = (0x80 + ((value >> 12) & 0x3F)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = (0x80 + ((value >> 6) & 0x3F)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                            *string.pointer = (0x80 + (value & 0x3F)) as yaml_char_t;
                            string.pointer = string.pointer.add(1);
                        }

                        /* Advance the pointer. */

                        k = 0;
                        while k < code_length {
                            crate::SKIP!(parser);
                            k += 1;
                        }
                    }
                } else {
                    /* It is a non-escaped non-blank character. */

                    if !crate::READ!(parser, string) {
                        break 'error;
                    }
                }

                if !crate::CACHE!(parser, 2) {
                    break 'error;
                }
            }

            /* Check if we are at the end of the scalar. */

            /* Fix for crash uninitialized value crash
             * Credit for the bug and input is to OSS Fuzz
             * Credit for the fix to Alex Gaynor
             */
            if !crate::CACHE!(parser, 1) {
                break 'error;
            }
            if crate::CHECK!((*parser).buffer, (if single != 0 { b'\'' } else { b'"' })) {
                break;
            }

            /* Consume blank characters. */

            if !crate::CACHE!(parser, 1) {
                break 'error;
            }

            while crate::IS_BLANK!((*parser).buffer) || crate::IS_BREAK!((*parser).buffer) {
                if crate::IS_BLANK!((*parser).buffer) {
                    /* Consume a space or a tab character. */

                    if leading_blanks == 0 {
                        if !crate::READ!(parser, whitespaces) {
                            break 'error;
                        }
                    } else {
                        crate::SKIP!(parser);
                    }
                } else {
                    if !crate::CACHE!(parser, 2) {
                        break 'error;
                    }

                    /* Check if it is a first line break. */

                    if leading_blanks == 0 {
                        crate::CLEAR!(parser, whitespaces);
                        if !crate::READ_LINE!(parser, leading_break) {
                            break 'error;
                        }
                        leading_blanks = 1;
                    } else {
                        if !crate::READ_LINE!(parser, trailing_breaks) {
                            break 'error;
                        }
                    }
                }
                if !crate::CACHE!(parser, 1) {
                    break 'error;
                }
            }

            /* Join the whitespaces or fold line breaks. */

            if leading_blanks != 0 {
                /* Do we need to fold line breaks? */

                if *leading_break.start.add(0) == b'\n' {
                    if *trailing_breaks.start.add(0) == b'\0' {
                        if !crate::STRING_EXTEND!(parser, string) {
                            break 'error;
                        }
                        *string.pointer = b' ';
                        string.pointer = string.pointer.add(1);
                    } else {
                        if !crate::JOIN!(parser, string, trailing_breaks) {
                            break 'error;
                        }
                        crate::CLEAR!(parser, trailing_breaks);
                    }
                    crate::CLEAR!(parser, leading_break);
                } else {
                    if !crate::JOIN!(parser, string, leading_break) {
                        break 'error;
                    }
                    if !crate::JOIN!(parser, string, trailing_breaks) {
                        break 'error;
                    }
                    crate::CLEAR!(parser, leading_break);
                    crate::CLEAR!(parser, trailing_breaks);
                }
            } else {
                if !crate::JOIN!(parser, string, whitespaces) {
                    break 'error;
                }
                crate::CLEAR!(parser, whitespaces);
            }
        }

        /* Eat the right quote. */

        crate::SKIP!(parser);

        end_mark = (*parser).mark;

        /* Create a token. */

        crate::SCALAR_TOKEN_INIT!(
            *token,
            string.start,
            string.pointer.offset_from(string.start) as size_t,
            if single != 0 {
                YAML_SINGLE_QUOTED_SCALAR_STYLE
            } else {
                YAML_DOUBLE_QUOTED_SCALAR_STYLE
            },
            start_mark,
            end_mark
        );

        crate::STRING_DEL!(parser, leading_break);
        crate::STRING_DEL!(parser, trailing_breaks);
        crate::STRING_DEL!(parser, whitespaces);

        return 1;
    }

    /* error: */

    crate::STRING_DEL!(parser, string);
    crate::STRING_DEL!(parser, leading_break);
    crate::STRING_DEL!(parser, trailing_breaks);
    crate::STRING_DEL!(parser, whitespaces);

    0
}

/*
 * Scan a plain scalar.
 */

pub(crate) unsafe fn yaml_parser_scan_plain_scalar(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
) -> c_int {
    let mut start_mark: yaml_mark_t;
    let mut end_mark: yaml_mark_t;
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut leading_break: yaml_string_t = crate::NULL_STRING!();
    let mut trailing_breaks: yaml_string_t = crate::NULL_STRING!();
    let mut whitespaces: yaml_string_t = crate::NULL_STRING!();
    let mut leading_blanks: c_int = 0;
    let indent: c_int = (*parser).indent + 1;

    start_mark = yaml_mark_t::zeroed();
    end_mark = yaml_mark_t::zeroed();

    'error: {
        if !crate::STRING_INIT!(parser, string, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }
        if !crate::STRING_INIT!(parser, leading_break, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }
        if !crate::STRING_INIT!(parser, trailing_breaks, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }
        if !crate::STRING_INIT!(parser, whitespaces, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }

        end_mark = (*parser).mark;
        start_mark = end_mark;

        /* Consume the content of the plain scalar. */

        loop {
            /* Check for a document indicator. */

            if !crate::CACHE!(parser, 4) {
                break 'error;
            }

            if (*parser).mark.column == 0
                && ((crate::CHECK_AT!((*parser).buffer, b'-', 0)
                    && crate::CHECK_AT!((*parser).buffer, b'-', 1)
                    && crate::CHECK_AT!((*parser).buffer, b'-', 2))
                    || (crate::CHECK_AT!((*parser).buffer, b'.', 0)
                        && crate::CHECK_AT!((*parser).buffer, b'.', 1)
                        && crate::CHECK_AT!((*parser).buffer, b'.', 2)))
                && crate::IS_BLANKZ_AT!((*parser).buffer, 3)
            {
                break;
            }

            /* Check for a comment. */

            if crate::CHECK!((*parser).buffer, b'#') {
                break;
            }

            /* Consume non-blank characters. */

            while !crate::IS_BLANKZ!((*parser).buffer) {
                /* Check for "x:" + one of ',?[]{}' in the flow context. TODO: Fix the test "spec-08-13".
                 * This is not completely according to the spec
                 * See http://yaml.org/spec/1.1/#id907281 9.1.3. Plain
                 */

                if (*parser).flow_level != 0
                    && crate::CHECK!((*parser).buffer, b':')
                    && (crate::CHECK_AT!((*parser).buffer, b',', 1)
                        || crate::CHECK_AT!((*parser).buffer, b'?', 1)
                        || crate::CHECK_AT!((*parser).buffer, b'[', 1)
                        || crate::CHECK_AT!((*parser).buffer, b']', 1)
                        || crate::CHECK_AT!((*parser).buffer, b'{', 1)
                        || crate::CHECK_AT!((*parser).buffer, b'}', 1))
                {
                    crate::scanner::yaml_parser_set_scanner_error(
                        parser,
                        b"while scanning a plain scalar\0".as_ptr() as *const c_char,
                        start_mark,
                        b"found unexpected ':'\0".as_ptr() as *const c_char,
                    );
                    break 'error;
                }

                /* Check for indicators that may end a plain scalar. */

                if (crate::CHECK!((*parser).buffer, b':')
                    && crate::IS_BLANKZ_AT!((*parser).buffer, 1))
                    || ((*parser).flow_level != 0
                        && (crate::CHECK!((*parser).buffer, b',')
                            || crate::CHECK!((*parser).buffer, b'[')
                            || crate::CHECK!((*parser).buffer, b']')
                            || crate::CHECK!((*parser).buffer, b'{')
                            || crate::CHECK!((*parser).buffer, b'}')))
                {
                    break;
                }

                /* Check if we need to join whitespaces and breaks. */

                if leading_blanks != 0 || whitespaces.start != whitespaces.pointer {
                    if leading_blanks != 0 {
                        /* Do we need to fold line breaks? */

                        if *leading_break.start.add(0) == b'\n' {
                            if *trailing_breaks.start.add(0) == b'\0' {
                                if !crate::STRING_EXTEND!(parser, string) {
                                    break 'error;
                                }
                                *string.pointer = b' ';
                                string.pointer = string.pointer.add(1);
                            } else {
                                if !crate::JOIN!(parser, string, trailing_breaks) {
                                    break 'error;
                                }
                                crate::CLEAR!(parser, trailing_breaks);
                            }
                            crate::CLEAR!(parser, leading_break);
                        } else {
                            if !crate::JOIN!(parser, string, leading_break) {
                                break 'error;
                            }
                            if !crate::JOIN!(parser, string, trailing_breaks) {
                                break 'error;
                            }
                            crate::CLEAR!(parser, leading_break);
                            crate::CLEAR!(parser, trailing_breaks);
                        }

                        leading_blanks = 0;
                    } else {
                        if !crate::JOIN!(parser, string, whitespaces) {
                            break 'error;
                        }
                        crate::CLEAR!(parser, whitespaces);
                    }
                }

                /* Copy the character. */

                if !crate::READ!(parser, string) {
                    break 'error;
                }

                end_mark = (*parser).mark;

                if !crate::CACHE!(parser, 2) {
                    break 'error;
                }
            }

            /* Is it the end? */

            if !(crate::IS_BLANK!((*parser).buffer) || crate::IS_BREAK!((*parser).buffer)) {
                break;
            }

            /* Consume blank characters. */

            if !crate::CACHE!(parser, 1) {
                break 'error;
            }

            while crate::IS_BLANK!((*parser).buffer) || crate::IS_BREAK!((*parser).buffer) {
                if crate::IS_BLANK!((*parser).buffer) {
                    /* Check for tab characters that abuse indentation. */

                    if leading_blanks != 0
                        && ((*parser).mark.column as c_int) < indent
                        && crate::IS_TAB!((*parser).buffer)
                    {
                        crate::scanner::yaml_parser_set_scanner_error(
                            parser,
                            b"while scanning a plain scalar\0".as_ptr() as *const c_char,
                            start_mark,
                            b"found a tab character that violates indentation\0".as_ptr()
                                as *const c_char,
                        );
                        break 'error;
                    }

                    /* Consume a space or a tab character. */

                    if leading_blanks == 0 {
                        if !crate::READ!(parser, whitespaces) {
                            break 'error;
                        }
                    } else {
                        crate::SKIP!(parser);
                    }
                } else {
                    if !crate::CACHE!(parser, 2) {
                        break 'error;
                    }

                    /* Check if it is a first line break. */

                    if leading_blanks == 0 {
                        crate::CLEAR!(parser, whitespaces);
                        if !crate::READ_LINE!(parser, leading_break) {
                            break 'error;
                        }
                        leading_blanks = 1;
                    } else {
                        if !crate::READ_LINE!(parser, trailing_breaks) {
                            break 'error;
                        }
                    }
                }
                if !crate::CACHE!(parser, 1) {
                    break 'error;
                }
            }

            /* Check indentation level. */

            if (*parser).flow_level == 0 && ((*parser).mark.column as c_int) < indent {
                break;
            }
        }

        /* Create a token. */

        crate::SCALAR_TOKEN_INIT!(
            *token,
            string.start,
            string.pointer.offset_from(string.start) as size_t,
            YAML_PLAIN_SCALAR_STYLE,
            start_mark,
            end_mark
        );

        /* Note that we change the 'simple_key_allowed' flag. */

        if leading_blanks != 0 {
            (*parser).simple_key_allowed = 1;
        }

        crate::STRING_DEL!(parser, leading_break);
        crate::STRING_DEL!(parser, trailing_breaks);
        crate::STRING_DEL!(parser, whitespaces);

        return 1;
    }

    /* error: */

    crate::STRING_DEL!(parser, string);
    crate::STRING_DEL!(parser, leading_break);
    crate::STRING_DEL!(parser, trailing_breaks);
    crate::STRING_DEL!(parser, whitespaces);

    0
}
