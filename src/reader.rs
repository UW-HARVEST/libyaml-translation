//! Translation of `c_src/src/reader.c`.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Set the reader error and return 0.
 */

pub(crate) unsafe fn yaml_parser_set_reader_error(
    parser: *mut yaml_parser_t,
    problem: *const c_char,
    offset: size_t,
    value: c_int,
) -> c_int {
    (*parser).error = YAML_READER_ERROR;
    (*parser).problem = problem;
    (*parser).problem_offset = offset;
    (*parser).problem_value = value;

    return 0;
}

/*
 * Byte order marks.
 */

const BOM_UTF8: &[u8; 4] = b"\xef\xbb\xbf\0";
const BOM_UTF16LE: &[u8; 3] = b"\xff\xfe\0";
const BOM_UTF16BE: &[u8; 3] = b"\xfe\xff\0";

/*
 * Determine the input stream encoding by checking the BOM symbol. If no BOM is
 * found, the UTF-8 encoding is assumed. Return 1 on success, 0 on failure.
 */

pub(crate) unsafe fn yaml_parser_determine_encoding(parser: *mut yaml_parser_t) -> c_int {
    /* Ensure that we had enough bytes in the raw buffer. */

    while (*parser).eof == 0
        && (*parser)
            .raw_buffer
            .last
            .offset_from((*parser).raw_buffer.pointer)
            < 3
    {
        if yaml_parser_update_raw_buffer(parser) == 0 {
            return 0;
        }
    }

    /* Determine the encoding. */

    if (*parser)
        .raw_buffer
        .last
        .offset_from((*parser).raw_buffer.pointer)
        >= 2
        && memcmp(
            (*parser).raw_buffer.pointer as *const c_void,
            BOM_UTF16LE.as_ptr() as *const c_void,
            2,
        ) == 0
    {
        (*parser).encoding = YAML_UTF16LE_ENCODING;
        (*parser).raw_buffer.pointer = (*parser).raw_buffer.pointer.add(2);
        (*parser).offset += 2;
    } else if (*parser)
        .raw_buffer
        .last
        .offset_from((*parser).raw_buffer.pointer)
        >= 2
        && memcmp(
            (*parser).raw_buffer.pointer as *const c_void,
            BOM_UTF16BE.as_ptr() as *const c_void,
            2,
        ) == 0
    {
        (*parser).encoding = YAML_UTF16BE_ENCODING;
        (*parser).raw_buffer.pointer = (*parser).raw_buffer.pointer.add(2);
        (*parser).offset += 2;
    } else if (*parser)
        .raw_buffer
        .last
        .offset_from((*parser).raw_buffer.pointer)
        >= 3
        && memcmp(
            (*parser).raw_buffer.pointer as *const c_void,
            BOM_UTF8.as_ptr() as *const c_void,
            3,
        ) == 0
    {
        (*parser).encoding = YAML_UTF8_ENCODING;
        (*parser).raw_buffer.pointer = (*parser).raw_buffer.pointer.add(3);
        (*parser).offset += 3;
    } else {
        (*parser).encoding = YAML_UTF8_ENCODING;
    }

    return 1;
}

/*
 * Update the raw buffer.
 */

pub(crate) unsafe fn yaml_parser_update_raw_buffer(parser: *mut yaml_parser_t) -> c_int {
    let mut size_read: size_t = 0;

    /* Return if the raw buffer is full. */

    if (*parser).raw_buffer.start == (*parser).raw_buffer.pointer
        && (*parser).raw_buffer.last == (*parser).raw_buffer.end
    {
        return 1;
    }

    /* Return on EOF. */

    if (*parser).eof != 0 {
        return 1;
    }

    /* Move the remaining bytes in the raw buffer to the beginning. */

    if (*parser).raw_buffer.start < (*parser).raw_buffer.pointer
        && (*parser).raw_buffer.pointer < (*parser).raw_buffer.last
    {
        memmove(
            (*parser).raw_buffer.start as *mut c_void,
            (*parser).raw_buffer.pointer as *const c_void,
            (*parser)
                .raw_buffer
                .last
                .offset_from((*parser).raw_buffer.pointer) as size_t,
        );
    }
    (*parser).raw_buffer.last = (*parser).raw_buffer.last.sub(
        (*parser)
            .raw_buffer
            .pointer
            .offset_from((*parser).raw_buffer.start) as size_t,
    );
    (*parser).raw_buffer.pointer = (*parser).raw_buffer.start;

    /* Call the read handler to fill the buffer. */

    if ((*parser).read_handler.unwrap())(
        (*parser).read_handler_data,
        (*parser).raw_buffer.last,
        (*parser)
            .raw_buffer
            .end
            .offset_from((*parser).raw_buffer.last) as size_t,
        &mut size_read,
    ) == 0
    {
        return yaml_parser_set_reader_error(
            parser,
            b"input error\0".as_ptr() as *const c_char,
            (*parser).offset,
            -1,
        );
    }
    (*parser).raw_buffer.last = (*parser).raw_buffer.last.add(size_read);
    if size_read == 0 {
        (*parser).eof = 1;
    }

    return 1;
}

/*
 * Ensure that the buffer contains at least `length` characters.
 * Return 1 on success, 0 on failure.
 *
 * The length is supposed to be significantly less that the buffer size.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_update_buffer(
    parser: *mut yaml_parser_t,
    length: size_t,
) -> c_int {
    let mut first: c_int = 1;

    yaml_assert!((*parser).read_handler.is_some()); /* Read handler must be set. */

    /* If the EOF flag is set and the raw buffer is empty, do nothing. */

    if (*parser).eof != 0 && (*parser).raw_buffer.pointer == (*parser).raw_buffer.last {
        return 1;
    }

    /* Return if the buffer contains enough characters. */

    if (*parser).unread >= length {
        return 1;
    }

    /* Determine the input encoding if it is not known yet. */

    if (*parser).encoding == 0 {
        if yaml_parser_determine_encoding(parser) == 0 {
            return 0;
        }
    }

    /* Move the unread characters to the beginning of the buffer. */

    if (*parser).buffer.start < (*parser).buffer.pointer
        && (*parser).buffer.pointer < (*parser).buffer.last
    {
        let size: size_t = (*parser).buffer.last.offset_from((*parser).buffer.pointer) as size_t;
        memmove(
            (*parser).buffer.start as *mut c_void,
            (*parser).buffer.pointer as *const c_void,
            size,
        );
        (*parser).buffer.pointer = (*parser).buffer.start;
        (*parser).buffer.last = (*parser).buffer.start.add(size);
    } else if (*parser).buffer.pointer == (*parser).buffer.last {
        (*parser).buffer.pointer = (*parser).buffer.start;
        (*parser).buffer.last = (*parser).buffer.start;
    }

    /* Fill the buffer until it has enough characters. */

    while (*parser).unread < length {
        /* Fill the raw buffer if necessary. */

        if first == 0 || (*parser).raw_buffer.pointer == (*parser).raw_buffer.last {
            if yaml_parser_update_raw_buffer(parser) == 0 {
                return 0;
            }
        }
        first = 0;

        /* Decode the raw buffer. */

        while (*parser).raw_buffer.pointer != (*parser).raw_buffer.last {
            let mut value: c_uint = 0;
            let mut value2: c_uint = 0;
            let mut incomplete: c_int = 0;
            let mut octet: c_uchar = 0;
            let mut width: c_uint = 0;
            let low: c_int;
            let high: c_int;
            let mut k: size_t;
            let raw_unread: size_t = (*parser)
                .raw_buffer
                .last
                .offset_from((*parser).raw_buffer.pointer)
                as size_t;

            /* Decode the next character. */

            'switch: {
                match (*parser).encoding {
                    YAML_UTF8_ENCODING => {
                        /*
                         * Decode a UTF-8 character.  Check RFC 3629
                         * (http://www.ietf.org/rfc/rfc3629.txt) for more details.
                         *
                         * The following table (taken from the RFC) is used for
                         * decoding.
                         *
                         *    Char. number range |        UTF-8 octet sequence
                         *      (hexadecimal)    |              (binary)
                         *   --------------------+------------------------------------
                         *   0000 0000-0000 007F | 0xxxxxxx
                         *   0000 0080-0000 07FF | 110xxxxx 10xxxxxx
                         *   0000 0800-0000 FFFF | 1110xxxx 10xxxxxx 10xxxxxx
                         *   0001 0000-0010 FFFF | 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
                         *
                         * Additionally, the characters in the range 0xD800-0xDFFF
                         * are prohibited as they are reserved for use with UTF-16
                         * surrogate pairs.
                         */

                        /* Determine the length of the UTF-8 sequence. */

                        octet = *(*parser).raw_buffer.pointer.add(0);
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

                        /* Check if the leading octet is valid. */

                        if width == 0 {
                            return yaml_parser_set_reader_error(
                                parser,
                                b"invalid leading UTF-8 octet\0".as_ptr() as *const c_char,
                                (*parser).offset,
                                octet as c_int,
                            );
                        }

                        /* Check if the raw buffer contains an incomplete character. */

                        if (width as size_t) > raw_unread {
                            if (*parser).eof != 0 {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    b"incomplete UTF-8 octet sequence\0".as_ptr() as *const c_char,
                                    (*parser).offset,
                                    -1,
                                );
                            }
                            incomplete = 1;
                            break 'switch;
                        }

                        /* Decode the leading octet. */

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

                        /* Check and decode the trailing octets. */

                        k = 1;
                        while k < width as size_t {
                            octet = *(*parser).raw_buffer.pointer.add(k);

                            /* Check if the octet is valid. */

                            if (octet & 0xC0) != 0x80 {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    b"invalid trailing UTF-8 octet\0".as_ptr() as *const c_char,
                                    (*parser).offset + k,
                                    octet as c_int,
                                );
                            }

                            /* Decode the octet. */

                            value = (value << 6) + (octet & 0x3F) as c_uint;

                            k += 1;
                        }

                        /* Check the length of the sequence against the value. */

                        if !((width == 1)
                            || (width == 2 && value >= 0x80)
                            || (width == 3 && value >= 0x800)
                            || (width == 4 && value >= 0x10000))
                        {
                            return yaml_parser_set_reader_error(
                                parser,
                                b"invalid length of a UTF-8 sequence\0".as_ptr() as *const c_char,
                                (*parser).offset,
                                -1,
                            );
                        }

                        /* Check the range of the value. */

                        if (value >= 0xD800 && value <= 0xDFFF) || value > 0x10FFFF {
                            return yaml_parser_set_reader_error(
                                parser,
                                b"invalid Unicode character\0".as_ptr() as *const c_char,
                                (*parser).offset,
                                value as c_int,
                            );
                        }

                        break 'switch;
                    }

                    YAML_UTF16LE_ENCODING | YAML_UTF16BE_ENCODING => {
                        low = if (*parser).encoding == YAML_UTF16LE_ENCODING {
                            0
                        } else {
                            1
                        };
                        high = if (*parser).encoding == YAML_UTF16LE_ENCODING {
                            1
                        } else {
                            0
                        };

                        /*
                         * The UTF-16 encoding is not as simple as one might
                         * naively think.  Check RFC 2781
                         * (http://www.ietf.org/rfc/rfc2781.txt).
                         *
                         * Normally, two subsequent bytes describe a Unicode
                         * character.  However a special technique (called a
                         * surrogate pair) is used for specifying character
                         * values larger than 0xFFFF.
                         *
                         * A surrogate pair consists of two pseudo-characters:
                         *      high surrogate area (0xD800-0xDBFF)
                         *      low surrogate area (0xDC00-0xDFFF)
                         *
                         * The following formulas are used for decoding
                         * and encoding characters using surrogate pairs:
                         *
                         *  U  = U' + 0x10000   (0x01 00 00 <= U <= 0x10 FF FF)
                         *  U' = yyyyyyyyyyxxxxxxxxxx   (0 <= U' <= 0x0F FF FF)
                         *  W1 = 110110yyyyyyyyyy
                         *  W2 = 110111xxxxxxxxxx
                         *
                         * where U is the character value, W1 is the high surrogate
                         * area, W2 is the low surrogate area.
                         */

                        /* Check for incomplete UTF-16 character. */

                        if raw_unread < 2 {
                            if (*parser).eof != 0 {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    b"incomplete UTF-16 character\0".as_ptr() as *const c_char,
                                    (*parser).offset,
                                    -1,
                                );
                            }
                            incomplete = 1;
                            break 'switch;
                        }

                        /* Get the character. */

                        value = *(*parser).raw_buffer.pointer.add(low as size_t) as c_uint
                            + ((*(*parser).raw_buffer.pointer.add(high as size_t) as c_uint) << 8);

                        /* Check for unexpected low surrogate area. */

                        if (value & 0xFC00) == 0xDC00 {
                            return yaml_parser_set_reader_error(
                                parser,
                                b"unexpected low surrogate area\0".as_ptr() as *const c_char,
                                (*parser).offset,
                                value as c_int,
                            );
                        }

                        /* Check for a high surrogate area. */

                        if (value & 0xFC00) == 0xD800 {
                            width = 4;

                            /* Check for incomplete surrogate pair. */

                            if raw_unread < 4 {
                                if (*parser).eof != 0 {
                                    return yaml_parser_set_reader_error(
                                        parser,
                                        b"incomplete UTF-16 surrogate pair\0".as_ptr()
                                            as *const c_char,
                                        (*parser).offset,
                                        -1,
                                    );
                                }
                                incomplete = 1;
                                break 'switch;
                            }

                            /* Get the next character. */

                            value2 = *(*parser).raw_buffer.pointer.add((low + 2) as size_t)
                                as c_uint
                                + ((*(*parser).raw_buffer.pointer.add((high + 2) as size_t)
                                    as c_uint)
                                    << 8);

                            /* Check for a low surrogate area. */

                            if (value2 & 0xFC00) != 0xDC00 {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    b"expected low surrogate area\0".as_ptr() as *const c_char,
                                    (*parser).offset + 2,
                                    value2 as c_int,
                                );
                            }

                            /* Generate the value of the surrogate pair. */

                            value = 0x10000 + ((value & 0x3FF) << 10) + (value2 & 0x3FF);
                        } else {
                            width = 2;
                        }

                        break 'switch;
                    }

                    _ => {
                        yaml_assert!(1 != 0); /* Impossible. */
                    }
                }
            }

            /* Check if the raw buffer contains enough bytes to form a character. */

            if incomplete != 0 {
                break;
            }

            /*
             * Check if the character is in the allowed range:
             *      #x9 | #xA | #xD | [#x20-#x7E]               (8 bit)
             *      | #x85 | [#xA0-#xD7FF] | [#xE000-#xFFFD]    (16 bit)
             *      | [#x10000-#x10FFFF]                        (32 bit)
             */

            if !(value == 0x09
                || value == 0x0A
                || value == 0x0D
                || (value >= 0x20 && value <= 0x7E)
                || (value == 0x85)
                || (value >= 0xA0 && value <= 0xD7FF)
                || (value >= 0xE000 && value <= 0xFFFD)
                || (value >= 0x10000 && value <= 0x10FFFF))
            {
                return yaml_parser_set_reader_error(
                    parser,
                    b"control characters are not allowed\0".as_ptr() as *const c_char,
                    (*parser).offset,
                    value as c_int,
                );
            }

            /* Move the raw pointers. */

            (*parser).raw_buffer.pointer = (*parser).raw_buffer.pointer.add(width as size_t);
            (*parser).offset += width as size_t;

            /* Finally put the character into the buffer. */

            /* 0000 0000-0000 007F -> 0xxxxxxx */
            if value <= 0x7F {
                *(*parser).buffer.last = value as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
            }
            /* 0000 0080-0000 07FF -> 110xxxxx 10xxxxxx */
            else if value <= 0x7FF {
                *(*parser).buffer.last = (0xC0 + (value >> 6)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
                *(*parser).buffer.last = (0x80 + (value & 0x3F)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
            }
            /* 0000 0800-0000 FFFF -> 1110xxxx 10xxxxxx 10xxxxxx */
            else if value <= 0xFFFF {
                *(*parser).buffer.last = (0xE0 + (value >> 12)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
                *(*parser).buffer.last = (0x80 + ((value >> 6) & 0x3F)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
                *(*parser).buffer.last = (0x80 + (value & 0x3F)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
            }
            /* 0001 0000-0010 FFFF -> 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
            else {
                *(*parser).buffer.last = (0xF0 + (value >> 18)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
                *(*parser).buffer.last = (0x80 + ((value >> 12) & 0x3F)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
                *(*parser).buffer.last = (0x80 + ((value >> 6) & 0x3F)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
                *(*parser).buffer.last = (0x80 + (value & 0x3F)) as yaml_char_t;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
            }

            (*parser).unread += 1;
        }

        /* On EOF, put NUL into the buffer and return. */

        if (*parser).eof != 0 {
            *(*parser).buffer.last = b'\0';
            (*parser).buffer.last = (*parser).buffer.last.add(1);
            (*parser).unread += 1;
            return 1;
        }
    }

    if (*parser).offset >= MAX_FILE_SIZE {
        return yaml_parser_set_reader_error(
            parser,
            b"input is too long\0".as_ptr() as *const c_char,
            (*parser).offset,
            -1,
        );
    }

    return 1;
}
