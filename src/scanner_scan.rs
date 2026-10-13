//! Translation of `c_src/src/scanner.c` lines 1938-2758: whitespace/comment
//! skipping, directives, anchors and tags.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Eat whitespaces and comments until the next token is found.
 */

pub(crate) unsafe fn yaml_parser_scan_to_next_token(parser: *mut yaml_parser_t) -> c_int {
    /* Until the next token is not found. */

    loop {
        /* Allow the BOM mark to start a line. */

        if !crate::CACHE!(parser, 1) {
            return 0;
        }

        if (*parser).mark.column == 0 && crate::IS_BOM!((*parser).buffer) {
            crate::SKIP!(parser);
        }

        /*
         * Eat whitespaces.
         *
         * Tabs are allowed:
         *
         *  - in the flow context;
         *  - in the block context, but not at the beginning of the line or
         *  after '-', '?', or ':' (complex value).
         */

        if !crate::CACHE!(parser, 1) {
            return 0;
        }

        while crate::CHECK!((*parser).buffer, b' ')
            || (((*parser).flow_level != 0 || (*parser).simple_key_allowed == 0)
                && crate::CHECK!((*parser).buffer, b'\t'))
        {
            crate::SKIP!(parser);
            if !crate::CACHE!(parser, 1) {
                return 0;
            }
        }

        /* Eat a comment until a line break. */

        if crate::CHECK!((*parser).buffer, b'#') {
            while !crate::IS_BREAKZ!((*parser).buffer) {
                crate::SKIP!(parser);
                if !crate::CACHE!(parser, 1) {
                    return 0;
                }
            }
        }

        /* If it is a line break, eat it. */

        if crate::IS_BREAK!((*parser).buffer) {
            if !crate::CACHE!(parser, 2) {
                return 0;
            }
            crate::SKIP_LINE!(parser);

            /* In the block context, a new line may start a simple key. */

            if (*parser).flow_level == 0 {
                (*parser).simple_key_allowed = 1;
            }
        } else {
            /* We have found a token. */

            break;
        }
    }

    1
}

/*
 * Scan a YAML-DIRECTIVE or TAG-DIRECTIVE token.
 *
 * Scope:
 *      %YAML    1.1    # a comment \n
 *      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 *      %TAG    !yaml!  tag:yaml.org,2002:  \n
 *      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 */

pub(crate) unsafe fn yaml_parser_scan_directive(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
) -> c_int {
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut name: *mut yaml_char_t = core::ptr::null_mut();
    let mut major: c_int = 0;
    let mut minor: c_int = 0;
    let mut handle: *mut yaml_char_t = core::ptr::null_mut();
    let mut prefix: *mut yaml_char_t = core::ptr::null_mut();

    /* Eat '%'. */

    start_mark = (*parser).mark;

    crate::SKIP!(parser);

    'error: {
        /* Scan the directive name. */

        if yaml_parser_scan_directive_name(parser, start_mark, &raw mut name) == 0 {
            break 'error;
        }

        /* Is it a YAML directive? */

        if strcmp(name as *const c_char, b"YAML\0".as_ptr() as *const c_char) == 0 {
            /* Scan the VERSION directive value. */

            if yaml_parser_scan_version_directive_value(
                parser,
                start_mark,
                &raw mut major,
                &raw mut minor,
            ) == 0
            {
                break 'error;
            }

            end_mark = (*parser).mark;

            /* Create a VERSION-DIRECTIVE token. */

            crate::VERSION_DIRECTIVE_TOKEN_INIT!((*token), major, minor, start_mark, end_mark);
        }
        /* Is it a TAG directive? */
        else if strcmp(name as *const c_char, b"TAG\0".as_ptr() as *const c_char) == 0 {
            /* Scan the TAG directive value. */

            if yaml_parser_scan_tag_directive_value(
                parser,
                start_mark,
                &raw mut handle,
                &raw mut prefix,
            ) == 0
            {
                break 'error;
            }

            end_mark = (*parser).mark;

            /* Create a TAG-DIRECTIVE token. */

            crate::TAG_DIRECTIVE_TOKEN_INIT!((*token), handle, prefix, start_mark, end_mark);
        }
        /* Unknown directive. */
        else {
            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                b"while scanning a directive\0".as_ptr() as *const c_char,
                start_mark,
                b"found unknown directive name\0".as_ptr() as *const c_char,
            );
            break 'error;
        }

        /* Eat the rest of the line including any comments. */

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
                b"while scanning a directive\0".as_ptr() as *const c_char,
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

        crate::memory::yaml_free(name as *mut c_void);

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(prefix as *mut c_void);
    crate::memory::yaml_free(handle as *mut c_void);
    crate::memory::yaml_free(name as *mut c_void);
    0
}

/*
 * Scan the directive name.
 *
 * Scope:
 *      %YAML   1.1     # a comment \n
 *       ^^^^
 *      %TAG    !yaml!  tag:yaml.org,2002:  \n
 *       ^^^
 */

pub(crate) unsafe fn yaml_parser_scan_directive_name(
    parser: *mut yaml_parser_t,
    start_mark: yaml_mark_t,
    name: *mut *mut yaml_char_t,
) -> c_int {
    let mut string = crate::NULL_STRING!();

    'error: {
        if !crate::STRING_INIT!(parser, string, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }

        /* Consume the directive name. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        while crate::IS_ALPHA!((*parser).buffer) {
            if !crate::READ!(parser, string) {
                break 'error;
            }
            if !crate::CACHE!(parser, 1) {
                break 'error;
            }
        }

        /* Check if the name is empty. */

        if string.start == string.pointer {
            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                b"while scanning a directive\0".as_ptr() as *const c_char,
                start_mark,
                b"could not find expected directive name\0".as_ptr() as *const c_char,
            );
            break 'error;
        }

        /* Check for an blank character after the name. */

        if !crate::IS_BLANKZ!((*parser).buffer) {
            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                b"while scanning a directive\0".as_ptr() as *const c_char,
                start_mark,
                b"found unexpected non-alphabetical character\0".as_ptr() as *const c_char,
            );
            break 'error;
        }

        *name = string.start;

        return 1;
    }

    /* error: */
    crate::STRING_DEL!(parser, string);
    0
}

/*
 * Scan the value of VERSION-DIRECTIVE.
 *
 * Scope:
 *      %YAML   1.1     # a comment \n
 *           ^^^^^^
 */

pub(crate) unsafe fn yaml_parser_scan_version_directive_value(
    parser: *mut yaml_parser_t,
    start_mark: yaml_mark_t,
    major: *mut c_int,
    minor: *mut c_int,
) -> c_int {
    /* Eat whitespaces. */

    if !crate::CACHE!(parser, 1) {
        return 0;
    }

    while crate::IS_BLANK!((*parser).buffer) {
        crate::SKIP!(parser);
        if !crate::CACHE!(parser, 1) {
            return 0;
        }
    }

    /* Consume the major version number. */

    if yaml_parser_scan_version_directive_number(parser, start_mark, major) == 0 {
        return 0;
    }

    /* Eat '.'. */

    if !crate::CHECK!((*parser).buffer, b'.') {
        return crate::scanner::yaml_parser_set_scanner_error(
            parser,
            b"while scanning a %YAML directive\0".as_ptr() as *const c_char,
            start_mark,
            b"did not find expected digit or '.' character\0".as_ptr() as *const c_char,
        );
    }

    crate::SKIP!(parser);

    /* Consume the minor version number. */

    if yaml_parser_scan_version_directive_number(parser, start_mark, minor) == 0 {
        return 0;
    }

    1
}

const MAX_NUMBER_LENGTH: c_int = 9;

/*
 * Scan the version number of VERSION-DIRECTIVE.
 *
 * Scope:
 *      %YAML   1.1     # a comment \n
 *              ^
 *      %YAML   1.1     # a comment \n
 *                ^
 */

pub(crate) unsafe fn yaml_parser_scan_version_directive_number(
    parser: *mut yaml_parser_t,
    start_mark: yaml_mark_t,
    number: *mut c_int,
) -> c_int {
    let mut value: c_int = 0;
    let mut length: size_t = 0;

    /* Repeat while the next character is digit. */

    if !crate::CACHE!(parser, 1) {
        return 0;
    }

    while crate::IS_DIGIT!((*parser).buffer) {
        /* Check if the number is too long. */

        length += 1;
        if length > MAX_NUMBER_LENGTH as size_t {
            return crate::scanner::yaml_parser_set_scanner_error(
                parser,
                b"while scanning a %YAML directive\0".as_ptr() as *const c_char,
                start_mark,
                b"found extremely long version number\0".as_ptr() as *const c_char,
            );
        }

        value = value * 10 + crate::AS_DIGIT!((*parser).buffer);

        crate::SKIP!(parser);

        if !crate::CACHE!(parser, 1) {
            return 0;
        }
    }

    /* Check if the number was present. */

    if length == 0 {
        return crate::scanner::yaml_parser_set_scanner_error(
            parser,
            b"while scanning a %YAML directive\0".as_ptr() as *const c_char,
            start_mark,
            b"did not find expected version number\0".as_ptr() as *const c_char,
        );
    }

    *number = value;

    1
}

/*
 * Scan the value of a TAG-DIRECTIVE token.
 *
 * Scope:
 *      %TAG    !yaml!  tag:yaml.org,2002:  \n
 *          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 */

pub(crate) unsafe fn yaml_parser_scan_tag_directive_value(
    parser: *mut yaml_parser_t,
    start_mark: yaml_mark_t,
    handle: *mut *mut yaml_char_t,
    prefix: *mut *mut yaml_char_t,
) -> c_int {
    let mut handle_value: *mut yaml_char_t = core::ptr::null_mut();
    let mut prefix_value: *mut yaml_char_t = core::ptr::null_mut();

    'error: {
        /* Eat whitespaces. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        while crate::IS_BLANK!((*parser).buffer) {
            crate::SKIP!(parser);
            if !crate::CACHE!(parser, 1) {
                break 'error;
            }
        }

        /* Scan a handle. */

        if yaml_parser_scan_tag_handle(parser, 1, start_mark, &raw mut handle_value) == 0 {
            break 'error;
        }

        /* Expect a whitespace. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        if !crate::IS_BLANK!((*parser).buffer) {
            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                b"while scanning a %TAG directive\0".as_ptr() as *const c_char,
                start_mark,
                b"did not find expected whitespace\0".as_ptr() as *const c_char,
            );
            break 'error;
        }

        /* Eat whitespaces. */

        while crate::IS_BLANK!((*parser).buffer) {
            crate::SKIP!(parser);
            if !crate::CACHE!(parser, 1) {
                break 'error;
            }
        }

        /* Scan a prefix. */

        if yaml_parser_scan_tag_uri(
            parser,
            1,
            1,
            core::ptr::null_mut(),
            start_mark,
            &raw mut prefix_value,
        ) == 0
        {
            break 'error;
        }

        /* Expect a whitespace or line break. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        if !crate::IS_BLANKZ!((*parser).buffer) {
            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                b"while scanning a %TAG directive\0".as_ptr() as *const c_char,
                start_mark,
                b"did not find expected whitespace or line break\0".as_ptr() as *const c_char,
            );
            break 'error;
        }

        *handle = handle_value;
        *prefix = prefix_value;

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(handle_value as *mut c_void);
    crate::memory::yaml_free(prefix_value as *mut c_void);
    0
}

pub(crate) unsafe fn yaml_parser_scan_anchor(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
    type_: yaml_token_type_t,
) -> c_int {
    let mut length: c_int = 0;
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;
    let mut string = crate::NULL_STRING!();

    'error: {
        if !crate::STRING_INIT!(parser, string, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }

        /* Eat the indicator character. */

        start_mark = (*parser).mark;

        crate::SKIP!(parser);

        /* Consume the value. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        while crate::IS_ALPHA!((*parser).buffer) {
            if !crate::READ!(parser, string) {
                break 'error;
            }
            if !crate::CACHE!(parser, 1) {
                break 'error;
            }
            length += 1;
        }

        end_mark = (*parser).mark;

        /*
         * Check if length of the anchor is greater than 0 and it is followed by
         * a whitespace character or one of the indicators:
         *
         *      '?', ':', ',', ']', '}', '%', '@', '`'.
         */

        if length == 0
            || !(crate::IS_BLANKZ!((*parser).buffer)
                || crate::CHECK!((*parser).buffer, b'?')
                || crate::CHECK!((*parser).buffer, b':')
                || crate::CHECK!((*parser).buffer, b',')
                || crate::CHECK!((*parser).buffer, b']')
                || crate::CHECK!((*parser).buffer, b'}')
                || crate::CHECK!((*parser).buffer, b'%')
                || crate::CHECK!((*parser).buffer, b'@')
                || crate::CHECK!((*parser).buffer, b'`'))
        {
            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                if type_ == YAML_ANCHOR_TOKEN {
                    b"while scanning an anchor\0".as_ptr() as *const c_char
                } else {
                    b"while scanning an alias\0".as_ptr() as *const c_char
                },
                start_mark,
                b"did not find expected alphabetic or numeric character\0".as_ptr()
                    as *const c_char,
            );
            break 'error;
        }

        /* Create a token. */

        if type_ == YAML_ANCHOR_TOKEN {
            crate::ANCHOR_TOKEN_INIT!((*token), string.start, start_mark, end_mark);
        } else {
            crate::ALIAS_TOKEN_INIT!((*token), string.start, start_mark, end_mark);
        }

        return 1;
    }

    /* error: */
    crate::STRING_DEL!(parser, string);
    0
}

/*
 * Scan a TAG token.
 */

pub(crate) unsafe fn yaml_parser_scan_tag(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
) -> c_int {
    let mut handle: *mut yaml_char_t = core::ptr::null_mut();
    let mut suffix: *mut yaml_char_t = core::ptr::null_mut();
    let start_mark: yaml_mark_t;
    let end_mark: yaml_mark_t;

    start_mark = (*parser).mark;

    'error: {
        /* Check if the tag is in the canonical form. */

        if !crate::CACHE!(parser, 2) {
            break 'error;
        }

        if crate::CHECK_AT!((*parser).buffer, b'<', 1) {
            /* Set the handle to '' */

            handle = crate::memory::yaml_malloc(1) as *mut yaml_char_t;
            if handle.is_null() {
                break 'error;
            }
            *handle.add(0) = b'\0';

            /* Eat '!<' */

            crate::SKIP!(parser);
            crate::SKIP!(parser);

            /* Consume the tag value. */

            if yaml_parser_scan_tag_uri(
                parser,
                1,
                0,
                core::ptr::null_mut(),
                start_mark,
                &raw mut suffix,
            ) == 0
            {
                break 'error;
            }

            /* Check for '>' and eat it. */

            if !crate::CHECK!((*parser).buffer, b'>') {
                crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a tag\0".as_ptr() as *const c_char,
                    start_mark,
                    b"did not find the expected '>'\0".as_ptr() as *const c_char,
                );
                break 'error;
            }

            crate::SKIP!(parser);
        } else {
            /* The tag has either the '!suffix' or the '!handle!suffix' form. */

            /* First, try to scan a handle. */

            if yaml_parser_scan_tag_handle(parser, 0, start_mark, &raw mut handle) == 0 {
                break 'error;
            }

            /* Check if it is, indeed, handle. */

            if *handle.add(0) == b'!'
                && *handle.add(1) != b'\0'
                && *handle.add(strlen(handle as *const c_char) - 1) == b'!'
            {
                /* Scan the suffix now. */

                if yaml_parser_scan_tag_uri(
                    parser,
                    0,
                    0,
                    core::ptr::null_mut(),
                    start_mark,
                    &raw mut suffix,
                ) == 0
                {
                    break 'error;
                }
            } else {
                /* It wasn't a handle after all.  Scan the rest of the tag. */

                if yaml_parser_scan_tag_uri(parser, 0, 0, handle, start_mark, &raw mut suffix) == 0
                {
                    break 'error;
                }

                /* Set the handle to '!'. */

                crate::memory::yaml_free(handle as *mut c_void);
                handle = crate::memory::yaml_malloc(2) as *mut yaml_char_t;
                if handle.is_null() {
                    break 'error;
                }
                *handle.add(0) = b'!';
                *handle.add(1) = b'\0';

                /*
                 * A special case: the '!' tag.  Set the handle to '' and the
                 * suffix to '!'.
                 */

                if *suffix.add(0) == b'\0' {
                    let tmp: *mut yaml_char_t = handle;
                    handle = suffix;
                    suffix = tmp;
                }
            }
        }

        /* Check the character which ends the tag. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        if !crate::IS_BLANKZ!((*parser).buffer) {
            if (*parser).flow_level == 0 || !crate::CHECK!((*parser).buffer, b',') {
                crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a tag\0".as_ptr() as *const c_char,
                    start_mark,
                    b"did not find expected whitespace or line break\0".as_ptr() as *const c_char,
                );
                break 'error;
            }
        }

        end_mark = (*parser).mark;

        /* Create a token. */

        crate::TAG_TOKEN_INIT!((*token), handle, suffix, start_mark, end_mark);

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(handle as *mut c_void);
    crate::memory::yaml_free(suffix as *mut c_void);
    0
}

/*
 * Scan a tag handle.
 */

pub(crate) unsafe fn yaml_parser_scan_tag_handle(
    parser: *mut yaml_parser_t,
    directive: c_int,
    start_mark: yaml_mark_t,
    handle: *mut *mut yaml_char_t,
) -> c_int {
    let mut string = crate::NULL_STRING!();

    'error: {
        if !crate::STRING_INIT!(parser, string, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }

        /* Check the initial '!' character. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        if !crate::CHECK!((*parser).buffer, b'!') {
            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                if directive != 0 {
                    b"while scanning a tag directive\0".as_ptr() as *const c_char
                } else {
                    b"while scanning a tag\0".as_ptr() as *const c_char
                },
                start_mark,
                b"did not find expected '!'\0".as_ptr() as *const c_char,
            );
            break 'error;
        }

        /* Copy the '!' character. */

        if !crate::READ!(parser, string) {
            break 'error;
        }

        /* Copy all subsequent alphabetical and numerical characters. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        while crate::IS_ALPHA!((*parser).buffer) {
            if !crate::READ!(parser, string) {
                break 'error;
            }
            if !crate::CACHE!(parser, 1) {
                break 'error;
            }
        }

        /* Check if the trailing character is '!' and copy it. */

        if crate::CHECK!((*parser).buffer, b'!') {
            if !crate::READ!(parser, string) {
                break 'error;
            }
        } else {
            /*
             * It's either the '!' tag or not really a tag handle.  If it's a %TAG
             * directive, it's an error.  If it's a tag token, it must be a part of
             * URI.
             */

            if directive != 0 && !(*string.start.add(0) == b'!' && *string.start.add(1) == b'\0') {
                crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    b"while parsing a tag directive\0".as_ptr() as *const c_char,
                    start_mark,
                    b"did not find expected '!'\0".as_ptr() as *const c_char,
                );
                break 'error;
            }
        }

        *handle = string.start;

        return 1;
    }

    /* error: */
    crate::STRING_DEL!(parser, string);
    0
}

/*
 * Scan a tag.
 */

pub(crate) unsafe fn yaml_parser_scan_tag_uri(
    parser: *mut yaml_parser_t,
    uri_char: c_int,
    directive: c_int,
    head: *mut yaml_char_t,
    start_mark: yaml_mark_t,
    uri: *mut *mut yaml_char_t,
) -> c_int {
    let mut length: size_t = if !head.is_null() {
        strlen(head as *const c_char)
    } else {
        0
    };
    let mut string = crate::NULL_STRING!();

    'error: {
        if !crate::STRING_INIT!(parser, string, crate::private::INITIAL_STRING_SIZE) {
            break 'error;
        }

        /* Resize the string to include the head. */

        while (string.end.offset_from(string.start) as size_t) <= length {
            if crate::memory::yaml_string_extend(
                &raw mut string.start,
                &raw mut string.pointer,
                &raw mut string.end,
            ) == 0
            {
                (*parser).error = YAML_MEMORY_ERROR;
                break 'error;
            }
        }

        /*
         * Copy the head if needed.
         *
         * Note that we don't copy the leading '!' character.
         */

        if length > 1 {
            memcpy(
                string.start as *mut c_void,
                head.add(1) as *const c_void,
                length - 1,
            );
            string.pointer = string.pointer.add(length - 1);
        }

        /* Scan the tag. */

        if !crate::CACHE!(parser, 1) {
            break 'error;
        }

        /*
         * The set of characters that may appear in URI is as follows:
         *
         *      '0'-'9', 'A'-'Z', 'a'-'z', '_', '-', ';', '/', '?', ':', '@', '&',
         *      '=', '+', '$', '.', '!', '~', '*', '\'', '(', ')', '%'.
         *
         * If we are inside a verbatim tag <...> (parameter uri_char is true)
         * then also the following flow indicators are allowed:
         *      ',', '[', ']'
         */

        while crate::IS_ALPHA!((*parser).buffer)
            || crate::CHECK!((*parser).buffer, b';')
            || crate::CHECK!((*parser).buffer, b'/')
            || crate::CHECK!((*parser).buffer, b'?')
            || crate::CHECK!((*parser).buffer, b':')
            || crate::CHECK!((*parser).buffer, b'@')
            || crate::CHECK!((*parser).buffer, b'&')
            || crate::CHECK!((*parser).buffer, b'=')
            || crate::CHECK!((*parser).buffer, b'+')
            || crate::CHECK!((*parser).buffer, b'$')
            || crate::CHECK!((*parser).buffer, b'.')
            || crate::CHECK!((*parser).buffer, b'%')
            || crate::CHECK!((*parser).buffer, b'!')
            || crate::CHECK!((*parser).buffer, b'~')
            || crate::CHECK!((*parser).buffer, b'*')
            || crate::CHECK!((*parser).buffer, b'\'')
            || crate::CHECK!((*parser).buffer, b'(')
            || crate::CHECK!((*parser).buffer, b')')
            || (uri_char != 0
                && (crate::CHECK!((*parser).buffer, b',')
                    || crate::CHECK!((*parser).buffer, b'[')
                    || crate::CHECK!((*parser).buffer, b']')))
        {
            /* Check if it is a URI-escape sequence. */

            if crate::CHECK!((*parser).buffer, b'%') {
                if !crate::STRING_EXTEND!(parser, string) {
                    break 'error;
                }

                if yaml_parser_scan_uri_escapes(parser, directive, start_mark, &raw mut string) == 0
                {
                    break 'error;
                }
            } else {
                if !crate::READ!(parser, string) {
                    break 'error;
                }
            }

            length += 1;
            if !crate::CACHE!(parser, 1) {
                break 'error;
            }
        }

        /* Check if the tag is non-empty. */

        if length == 0 {
            if !crate::STRING_EXTEND!(parser, string) {
                break 'error;
            }

            crate::scanner::yaml_parser_set_scanner_error(
                parser,
                if directive != 0 {
                    b"while parsing a %TAG directive\0".as_ptr() as *const c_char
                } else {
                    b"while parsing a tag\0".as_ptr() as *const c_char
                },
                start_mark,
                b"did not find expected tag URI\0".as_ptr() as *const c_char,
            );
            break 'error;
        }

        *uri = string.start;

        return 1;
    }

    /* error: */
    crate::STRING_DEL!(parser, string);
    0
}

/*
 * Decode an URI-escape sequence corresponding to a single UTF-8 character.
 */

pub(crate) unsafe fn yaml_parser_scan_uri_escapes(
    parser: *mut yaml_parser_t,
    directive: c_int,
    start_mark: yaml_mark_t,
    string: *mut yaml_string_t,
) -> c_int {
    let mut width: c_int = 0;

    /* Decode the required number of characters. */

    loop {
        let octet: c_uchar;

        /* Check for a URI-escaped octet. */

        if !crate::CACHE!(parser, 3) {
            return 0;
        }

        if !(crate::CHECK!((*parser).buffer, b'%')
            && crate::IS_HEX_AT!((*parser).buffer, 1)
            && crate::IS_HEX_AT!((*parser).buffer, 2))
        {
            return crate::scanner::yaml_parser_set_scanner_error(
                parser,
                if directive != 0 {
                    b"while parsing a %TAG directive\0".as_ptr() as *const c_char
                } else {
                    b"while parsing a tag\0".as_ptr() as *const c_char
                },
                start_mark,
                b"did not find URI escaped octet\0".as_ptr() as *const c_char,
            );
        }

        /* Get the octet. */

        octet = ((crate::AS_HEX_AT!((*parser).buffer, 1) << 4)
            + crate::AS_HEX_AT!((*parser).buffer, 2)) as c_uchar;

        /* If it is the leading octet, determine the length of the UTF-8 sequence. */

        if width == 0 {
            width = if (octet & 0x80) == 0x00 {
                1
            } else if (octet & 0xE0) == 0xC0 {
                2
            } else if (octet & 0xF0) == 0xE0 {
                3
            } else if (octet & 0xF8) == 0xF0 {
                4
            } else {
                0
            };
            if width == 0 {
                return crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    if directive != 0 {
                        b"while parsing a %TAG directive\0".as_ptr() as *const c_char
                    } else {
                        b"while parsing a tag\0".as_ptr() as *const c_char
                    },
                    start_mark,
                    b"found an incorrect leading UTF-8 octet\0".as_ptr() as *const c_char,
                );
            }
        } else {
            /* Check if the trailing octet is correct. */

            if (octet & 0xC0) != 0x80 {
                return crate::scanner::yaml_parser_set_scanner_error(
                    parser,
                    if directive != 0 {
                        b"while parsing a %TAG directive\0".as_ptr() as *const c_char
                    } else {
                        b"while parsing a tag\0".as_ptr() as *const c_char
                    },
                    start_mark,
                    b"found an incorrect trailing UTF-8 octet\0".as_ptr() as *const c_char,
                );
            }
        }

        /* Copy the octet and move the pointers. */

        *(*string).pointer = octet;
        (*string).pointer = (*string).pointer.add(1);
        crate::SKIP!(parser);
        crate::SKIP!(parser);
        crate::SKIP!(parser);

        width -= 1;
        if width == 0 {
            break;
        }
    }

    1
}
