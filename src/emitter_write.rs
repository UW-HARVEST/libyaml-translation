//! Translation of `c_src/src/emitter.c` lines 1776-2358: the low-level
//! `yaml_emitter_write_*` output routines.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Write the BOM character.
 */

pub(crate) unsafe fn yaml_emitter_write_bom(emitter: *mut yaml_emitter_t) -> c_int {
    if !crate::FLUSH!(emitter) {
        return 0;
    }

    *(*emitter).buffer.pointer = 0xEF as yaml_char_t;
    (*emitter).buffer.pointer = (*emitter).buffer.pointer.add(1);
    *(*emitter).buffer.pointer = 0xBB as yaml_char_t;
    (*emitter).buffer.pointer = (*emitter).buffer.pointer.add(1);
    *(*emitter).buffer.pointer = 0xBF as yaml_char_t;
    (*emitter).buffer.pointer = (*emitter).buffer.pointer.add(1);

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_indent(emitter: *mut yaml_emitter_t) -> c_int {
    let indent: c_int = if (*emitter).indent >= 0 {
        (*emitter).indent
    } else {
        0
    };

    if (*emitter).indention == 0
        || (*emitter).column > indent
        || ((*emitter).column == indent && (*emitter).whitespace == 0)
    {
        if !crate::PUT_BREAK!(emitter) {
            return 0;
        }
    }

    while (*emitter).column < indent {
        if !crate::PUT!(emitter, b' ') {
            return 0;
        }
    }

    (*emitter).whitespace = 1;
    (*emitter).indention = 1;

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_indicator(
    emitter: *mut yaml_emitter_t,
    indicator: *const c_char,
    need_whitespace: c_int,
    is_whitespace: c_int,
    is_indention: c_int,
) -> c_int {
    let indicator_length: size_t;
    let mut string: yaml_string_t = crate::NULL_STRING!();

    indicator_length = crate::libc_shim::strlen(indicator);
    crate::STRING_ASSIGN!(string, indicator as *mut yaml_char_t, indicator_length);

    if need_whitespace != 0 && (*emitter).whitespace == 0 {
        if !crate::PUT!(emitter, b' ') {
            return 0;
        }
    }

    while string.pointer != string.end {
        if !crate::WRITE!(emitter, string) {
            return 0;
        }
    }

    (*emitter).whitespace = is_whitespace;
    (*emitter).indention = ((*emitter).indention != 0 && is_indention != 0) as c_int;

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_anchor(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();
    crate::STRING_ASSIGN!(string, value, length);

    while string.pointer != string.end {
        if !crate::WRITE!(emitter, string) {
            return 0;
        }
    }

    (*emitter).whitespace = 0;
    (*emitter).indention = 0;

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_tag_handle(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();
    crate::STRING_ASSIGN!(string, value, length);

    if (*emitter).whitespace == 0 {
        if !crate::PUT!(emitter, b' ') {
            return 0;
        }
    }

    while string.pointer != string.end {
        if !crate::WRITE!(emitter, string) {
            return 0;
        }
    }

    (*emitter).whitespace = 0;
    (*emitter).indention = 0;

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_tag_content(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
    need_whitespace: c_int,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();
    crate::STRING_ASSIGN!(string, value, length);

    if need_whitespace != 0 && (*emitter).whitespace == 0 {
        if !crate::PUT!(emitter, b' ') {
            return 0;
        }
    }

    while string.pointer != string.end {
        if crate::IS_ALPHA!(string)
            || crate::CHECK!(string, b';')
            || crate::CHECK!(string, b'/')
            || crate::CHECK!(string, b'?')
            || crate::CHECK!(string, b':')
            || crate::CHECK!(string, b'@')
            || crate::CHECK!(string, b'&')
            || crate::CHECK!(string, b'=')
            || crate::CHECK!(string, b'+')
            || crate::CHECK!(string, b'$')
            || crate::CHECK!(string, b',')
            || crate::CHECK!(string, b'_')
            || crate::CHECK!(string, b'.')
            || crate::CHECK!(string, b'~')
            || crate::CHECK!(string, b'*')
            || crate::CHECK!(string, b'\'')
            || crate::CHECK!(string, b'(')
            || crate::CHECK!(string, b')')
            || crate::CHECK!(string, b'[')
            || crate::CHECK!(string, b']')
        {
            if !crate::WRITE!(emitter, string) {
                return 0;
            }
        } else {
            let mut width: c_int = crate::WIDTH!(string);
            let mut value: c_uint;
            /* C: `while (width --)` -- test the old value, then decrement. */
            loop {
                let old_width = width;
                width -= 1;
                if old_width == 0 {
                    break;
                }
                value = *string.pointer as c_uint;
                string.pointer = string.pointer.add(1);
                if !crate::PUT!(emitter, b'%') {
                    return 0;
                }
                if !crate::PUT!(
                    emitter,
                    (value >> 4)
                        + (if (value >> 4) < 10 {
                            b'0' as c_uint
                        } else {
                            b'A' as c_uint - 10
                        })
                ) {
                    return 0;
                }
                if !crate::PUT!(
                    emitter,
                    (value & 0x0F)
                        + (if (value & 0x0F) < 10 {
                            b'0' as c_uint
                        } else {
                            b'A' as c_uint - 10
                        })
                ) {
                    return 0;
                }
            }
        }
    }

    (*emitter).whitespace = 0;
    (*emitter).indention = 0;

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_plain_scalar(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
    allow_breaks: c_int,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut spaces: c_int = 0;
    let mut breaks: c_int = 0;

    crate::STRING_ASSIGN!(string, value, length);

    /*
     * Avoid trailing spaces for empty values in block mode.
     * In flow mode, we still want the space to prevent ambiguous things
     * like {a:}.
     * Currently, the emitter forbids any plain empty scalar in flow mode
     * (e.g. it outputs {a: ''} instead), so emitter->flow_level will
     * never be true here.
     * But if the emitter is ever changed to allow emitting empty values,
     * the check for flow_level is already here.
     */
    if (*emitter).whitespace == 0 && (length != 0 || (*emitter).flow_level != 0) {
        if !crate::PUT!(emitter, b' ') {
            return 0;
        }
    }

    while string.pointer != string.end {
        if crate::IS_SPACE!(string) {
            if allow_breaks != 0
                && spaces == 0
                && (*emitter).column > (*emitter).best_width
                && !crate::IS_SPACE_AT!(string, 1)
            {
                if yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
                crate::MOVE!(string);
            } else {
                if !crate::WRITE!(emitter, string) {
                    return 0;
                }
            }
            spaces = 1;
        } else if crate::IS_BREAK!(string) {
            if breaks == 0 && crate::CHECK!(string, b'\n') {
                if !crate::PUT_BREAK!(emitter) {
                    return 0;
                }
            }
            if !crate::WRITE_BREAK!(emitter, string) {
                return 0;
            }
            (*emitter).indention = 1;
            breaks = 1;
        } else {
            if breaks != 0 {
                if yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
            }
            if !crate::WRITE!(emitter, string) {
                return 0;
            }
            (*emitter).indention = 0;
            spaces = 0;
            breaks = 0;
        }
    }

    (*emitter).whitespace = 0;
    (*emitter).indention = 0;

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_single_quoted_scalar(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
    allow_breaks: c_int,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut spaces: c_int = 0;
    let mut breaks: c_int = 0;

    crate::STRING_ASSIGN!(string, value, length);

    if yaml_emitter_write_indicator(emitter, b"'\0".as_ptr() as *const c_char, 1, 0, 0) == 0 {
        return 0;
    }

    while string.pointer != string.end {
        if crate::IS_SPACE!(string) {
            if allow_breaks != 0
                && spaces == 0
                && (*emitter).column > (*emitter).best_width
                && string.pointer != string.start
                && string.pointer != string.end.sub(1)
                && !crate::IS_SPACE_AT!(string, 1)
            {
                if yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
                crate::MOVE!(string);
            } else {
                if !crate::WRITE!(emitter, string) {
                    return 0;
                }
            }
            spaces = 1;
        } else if crate::IS_BREAK!(string) {
            if breaks == 0 && crate::CHECK!(string, b'\n') {
                if !crate::PUT_BREAK!(emitter) {
                    return 0;
                }
            }
            if !crate::WRITE_BREAK!(emitter, string) {
                return 0;
            }
            (*emitter).indention = 1;
            breaks = 1;
        } else {
            if breaks != 0 {
                if yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
            }
            if crate::CHECK!(string, b'\'') {
                if !crate::PUT!(emitter, b'\'') {
                    return 0;
                }
            }
            if !crate::WRITE!(emitter, string) {
                return 0;
            }
            (*emitter).indention = 0;
            spaces = 0;
            breaks = 0;
        }
    }

    if breaks != 0 {
        if yaml_emitter_write_indent(emitter) == 0 {
            return 0;
        }
    }

    if yaml_emitter_write_indicator(emitter, b"'\0".as_ptr() as *const c_char, 0, 0, 0) == 0 {
        return 0;
    }

    (*emitter).whitespace = 0;
    (*emitter).indention = 0;

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_double_quoted_scalar(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
    allow_breaks: c_int,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut spaces: c_int = 0;

    crate::STRING_ASSIGN!(string, value, length);

    if yaml_emitter_write_indicator(emitter, b"\"\0".as_ptr() as *const c_char, 1, 0, 0) == 0 {
        return 0;
    }

    while string.pointer != string.end {
        if !crate::IS_PRINTABLE!(string)
            || ((*emitter).unicode == 0 && !crate::IS_ASCII!(string))
            || crate::IS_BOM!(string)
            || crate::IS_BREAK!(string)
            || crate::CHECK!(string, b'"')
            || crate::CHECK!(string, b'\\')
        {
            let mut octet: c_uchar;
            let mut width: c_uint;
            let mut value: c_uint;
            let mut k: c_int;

            octet = *string.pointer.add(0);
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
            value = if (octet & 0x80) == 0x00 {
                (octet & 0x7F) as c_uint
            } else if (octet & 0xE0) == 0xC0 {
                (octet & 0x1F) as c_uint
            } else if (octet & 0xF0) == 0xE0 {
                (octet & 0x0F) as c_uint
            } else if (octet & 0xF8) == 0xF0 {
                (octet & 0x07) as c_uint
            } else {
                0
            };
            k = 1;
            while k < width as c_int {
                octet = *string.pointer.add(k as usize);
                value = (value << 6) + (octet & 0x3F) as c_uint;
                k += 1;
            }
            string.pointer = string.pointer.add(width as usize);

            if !crate::PUT!(emitter, b'\\') {
                return 0;
            }

            match value {
                0x00 => {
                    if !crate::PUT!(emitter, b'0') {
                        return 0;
                    }
                }

                0x07 => {
                    if !crate::PUT!(emitter, b'a') {
                        return 0;
                    }
                }

                0x08 => {
                    if !crate::PUT!(emitter, b'b') {
                        return 0;
                    }
                }

                0x09 => {
                    if !crate::PUT!(emitter, b't') {
                        return 0;
                    }
                }

                0x0A => {
                    if !crate::PUT!(emitter, b'n') {
                        return 0;
                    }
                }

                0x0B => {
                    if !crate::PUT!(emitter, b'v') {
                        return 0;
                    }
                }

                0x0C => {
                    if !crate::PUT!(emitter, b'f') {
                        return 0;
                    }
                }

                0x0D => {
                    if !crate::PUT!(emitter, b'r') {
                        return 0;
                    }
                }

                0x1B => {
                    if !crate::PUT!(emitter, b'e') {
                        return 0;
                    }
                }

                0x22 => {
                    if !crate::PUT!(emitter, b'\"') {
                        return 0;
                    }
                }

                0x5C => {
                    if !crate::PUT!(emitter, b'\\') {
                        return 0;
                    }
                }

                0x85 => {
                    if !crate::PUT!(emitter, b'N') {
                        return 0;
                    }
                }

                0xA0 => {
                    if !crate::PUT!(emitter, b'_') {
                        return 0;
                    }
                }

                0x2028 => {
                    if !crate::PUT!(emitter, b'L') {
                        return 0;
                    }
                }

                0x2029 => {
                    if !crate::PUT!(emitter, b'P') {
                        return 0;
                    }
                }

                _ => {
                    if value <= 0xFF {
                        if !crate::PUT!(emitter, b'x') {
                            return 0;
                        }
                        width = 2;
                    } else if value <= 0xFFFF {
                        if !crate::PUT!(emitter, b'u') {
                            return 0;
                        }
                        width = 4;
                    } else {
                        if !crate::PUT!(emitter, b'U') {
                            return 0;
                        }
                        width = 8;
                    }
                    /*
                     * Emit the code point as uppercase hex, most significant
                     * nibble first, exactly `width` digits -- this is what the
                     * C source does by hand (no printf involved).
                     */
                    k = ((width - 1) * 4) as c_int;
                    while k >= 0 {
                        let digit: c_int = ((value >> (k as c_uint)) & 0x0F) as c_int;
                        if !crate::PUT!(
                            emitter,
                            digit
                                + (if digit < 10 {
                                    b'0' as c_int
                                } else {
                                    b'A' as c_int - 10
                                })
                        ) {
                            return 0;
                        }
                        k -= 4;
                    }
                }
            }
            spaces = 0;
        } else if crate::IS_SPACE!(string) {
            if allow_breaks != 0
                && spaces == 0
                && (*emitter).column > (*emitter).best_width
                && string.pointer != string.start
                && string.pointer != string.end.sub(1)
            {
                if yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
                if crate::IS_SPACE_AT!(string, 1) {
                    if !crate::PUT!(emitter, b'\\') {
                        return 0;
                    }
                }
                crate::MOVE!(string);
            } else {
                if !crate::WRITE!(emitter, string) {
                    return 0;
                }
            }
            spaces = 1;
        } else {
            if !crate::WRITE!(emitter, string) {
                return 0;
            }
            spaces = 0;
        }
    }

    if yaml_emitter_write_indicator(emitter, b"\"\0".as_ptr() as *const c_char, 0, 0, 0) == 0 {
        return 0;
    }

    (*emitter).whitespace = 0;
    (*emitter).indention = 0;

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_block_scalar_hints(
    emitter: *mut yaml_emitter_t,
    mut string: yaml_string_t,
) -> c_int {
    let mut indent_hint: [c_char; 2] = [0; 2];
    let mut chomp_hint: *const c_char = core::ptr::null();

    if crate::IS_SPACE!(string) || crate::IS_BREAK!(string) {
        indent_hint[0] = (b'0' as c_char).wrapping_add((*emitter).best_indent as c_char);
        indent_hint[1] = b'\0' as c_char;
        if yaml_emitter_write_indicator(emitter, indent_hint.as_ptr(), 0, 0, 0) == 0 {
            return 0;
        }
    }

    (*emitter).open_ended = 0;

    string.pointer = string.end;
    if string.start == string.pointer {
        chomp_hint = b"-\0".as_ptr() as *const c_char;
    } else {
        loop {
            string.pointer = string.pointer.sub(1);
            if (*string.pointer & 0xC0) != 0x80 {
                break;
            }
        }
        if !crate::IS_BREAK!(string) {
            chomp_hint = b"-\0".as_ptr() as *const c_char;
        } else if string.start == string.pointer {
            chomp_hint = b"+\0".as_ptr() as *const c_char;
            (*emitter).open_ended = 2;
        } else {
            loop {
                string.pointer = string.pointer.sub(1);
                if (*string.pointer & 0xC0) != 0x80 {
                    break;
                }
            }
            if crate::IS_BREAK!(string) {
                chomp_hint = b"+\0".as_ptr() as *const c_char;
                (*emitter).open_ended = 2;
            }
        }
    }

    if !chomp_hint.is_null() {
        if yaml_emitter_write_indicator(emitter, chomp_hint, 0, 0, 0) == 0 {
            return 0;
        }
    }

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_literal_scalar(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut breaks: c_int = 1;

    crate::STRING_ASSIGN!(string, value, length);

    if yaml_emitter_write_indicator(emitter, b"|\0".as_ptr() as *const c_char, 1, 0, 0) == 0 {
        return 0;
    }
    if yaml_emitter_write_block_scalar_hints(emitter, string) == 0 {
        return 0;
    }
    if !crate::PUT_BREAK!(emitter) {
        return 0;
    }
    (*emitter).indention = 1;
    (*emitter).whitespace = 1;

    while string.pointer != string.end {
        if crate::IS_BREAK!(string) {
            if !crate::WRITE_BREAK!(emitter, string) {
                return 0;
            }
            (*emitter).indention = 1;
            breaks = 1;
        } else {
            if breaks != 0 {
                if yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
            }
            if !crate::WRITE!(emitter, string) {
                return 0;
            }
            (*emitter).indention = 0;
            breaks = 0;
        }
    }

    return 1;
}

pub(crate) unsafe fn yaml_emitter_write_folded_scalar(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut breaks: c_int = 1;
    let mut leading_spaces: c_int = 1;

    crate::STRING_ASSIGN!(string, value, length);

    if yaml_emitter_write_indicator(emitter, b">\0".as_ptr() as *const c_char, 1, 0, 0) == 0 {
        return 0;
    }
    if yaml_emitter_write_block_scalar_hints(emitter, string) == 0 {
        return 0;
    }
    if !crate::PUT_BREAK!(emitter) {
        return 0;
    }
    (*emitter).indention = 1;
    (*emitter).whitespace = 1;

    while string.pointer != string.end {
        if crate::IS_BREAK!(string) {
            if breaks == 0 && leading_spaces == 0 && crate::CHECK!(string, b'\n') {
                let mut k: c_int = 0;
                while crate::IS_BREAK_AT!(string, k) {
                    k += crate::WIDTH_AT!(string, k);
                }
                if !crate::IS_BLANKZ_AT!(string, k) {
                    if !crate::PUT_BREAK!(emitter) {
                        return 0;
                    }
                }
            }
            if !crate::WRITE_BREAK!(emitter, string) {
                return 0;
            }
            (*emitter).indention = 1;
            breaks = 1;
        } else {
            if breaks != 0 {
                if yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
                leading_spaces = crate::IS_BLANK!(string) as c_int;
            }
            if breaks == 0
                && crate::IS_SPACE!(string)
                && !crate::IS_SPACE_AT!(string, 1)
                && (*emitter).column > (*emitter).best_width
            {
                if yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
                crate::MOVE!(string);
            } else {
                if !crate::WRITE!(emitter, string) {
                    return 0;
                }
            }
            (*emitter).indention = 0;
            breaks = 0;
        }
    }

    return 1;
}
