//! Translation of `c_src/src/api.c` lines 583-1033: token/event constructors
//! and destructors.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/// The anonymous `struct { yaml_error_type_t error; } context;` that several
/// functions in this range declare on the stack purely so that the
/// `STACK_INIT`/`PUSH`/`POP`/`STACK_DEL` macros have somewhere to write an
/// error code.  The value is never read back by the C code.
#[repr(C)]
pub(crate) struct ContextErr {
    pub error: yaml_error_type_t,
}

/*
 * Destroy a token object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_token_delete(token: *mut yaml_token_t) {
    yaml_assert!(!token.is_null()); /* Non-NULL token object expected. */

    match (*token).type_ {
        YAML_TAG_DIRECTIVE_TOKEN => {
            crate::memory::yaml_free((*token).data.tag_directive.handle as *mut c_void);
            crate::memory::yaml_free((*token).data.tag_directive.prefix as *mut c_void);
        }

        YAML_ALIAS_TOKEN => {
            crate::memory::yaml_free((*token).data.alias.value as *mut c_void);
        }

        YAML_ANCHOR_TOKEN => {
            crate::memory::yaml_free((*token).data.anchor.value as *mut c_void);
        }

        YAML_TAG_TOKEN => {
            crate::memory::yaml_free((*token).data.tag.handle as *mut c_void);
            crate::memory::yaml_free((*token).data.tag.suffix as *mut c_void);
        }

        YAML_SCALAR_TOKEN => {
            crate::memory::yaml_free((*token).data.scalar.value as *mut c_void);
        }

        _ => {}
    }

    memset(
        token as *mut c_void,
        0,
        core::mem::size_of::<yaml_token_t>() as size_t,
    );
}

/*
 * Check if a string is a valid UTF-8 sequence.
 *
 * Check 'reader.c' for more details on UTF-8 encoding.
 */

pub(crate) unsafe fn yaml_check_utf8(start: *const yaml_char_t, length: size_t) -> c_int {
    let end: *const yaml_char_t = start.add(length);
    let mut pointer: *const yaml_char_t = start;

    while pointer < end {
        let mut octet: c_uchar;
        let width: c_uint;
        let mut value: c_uint;
        let mut k: size_t;

        octet = *pointer.add(0);
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
        if width == 0 {
            return 0;
        }
        if pointer.wrapping_add(width as usize) > end {
            return 0;
        }
        k = 1;
        while k < width as size_t {
            octet = *pointer.add(k);
            if (octet & 0xC0) != 0x80 {
                return 0;
            }
            value = (value << 6) + (octet & 0x3F) as c_uint;
            k += 1;
        }
        if !((width == 1)
            || (width == 2 && value >= 0x80)
            || (width == 3 && value >= 0x800)
            || (width == 4 && value >= 0x10000))
        {
            return 0;
        }

        pointer = pointer.add(width as usize);
    }

    1
}

/*
 * Create STREAM-START.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_stream_start_event_initialize(
    event: *mut yaml_event_t,
    encoding: yaml_encoding_t,
) -> c_int {
    let mark = yaml_mark_t::zeroed();

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */

    crate::STREAM_START_EVENT_INIT!((*event), encoding, mark, mark);

    return 1;
}

/*
 * Create STREAM-END.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_stream_end_event_initialize(event: *mut yaml_event_t) -> c_int {
    let mark = yaml_mark_t::zeroed();

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */

    crate::STREAM_END_EVENT_INIT!((*event), mark, mark);

    return 1;
}

/*
 * Create DOCUMENT-START.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_start_event_initialize(
    event: *mut yaml_event_t,
    version_directive: *mut yaml_version_directive_t,
    tag_directives_start: *mut yaml_tag_directive_t,
    tag_directives_end: *mut yaml_tag_directive_t,
    implicit: c_int,
) -> c_int {
    let mut context = ContextErr {
        error: YAML_NO_ERROR,
    };
    let context = &raw mut context;
    let mark = yaml_mark_t::zeroed();
    let mut version_directive_copy: *mut yaml_version_directive_t = core::ptr::null_mut();
    let mut tag_directives_copy: yaml_stack_t<yaml_tag_directive_t> = yaml_stack_t::zeroed();
    let mut value = yaml_tag_directive_t {
        handle: core::ptr::null_mut(),
        prefix: core::ptr::null_mut(),
    };

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */
    yaml_assert!(
        (!tag_directives_start.is_null() && !tag_directives_end.is_null())
            || (tag_directives_start == tag_directives_end)
    );
    /* Valid tag directives are expected. */

    'error: {
        if !version_directive.is_null() {
            version_directive_copy = crate::memory::yaml_malloc(
                core::mem::size_of::<yaml_version_directive_t>() as size_t,
            ) as *mut yaml_version_directive_t;
            if version_directive_copy.is_null() {
                break 'error;
            }
            (*version_directive_copy).major = (*version_directive).major;
            (*version_directive_copy).minor = (*version_directive).minor;
        }

        if tag_directives_start != tag_directives_end {
            let mut tag_directive: *mut yaml_tag_directive_t;
            if !crate::STACK_INIT!(context, tag_directives_copy) {
                break 'error;
            }
            tag_directive = tag_directives_start;
            while tag_directive != tag_directives_end {
                yaml_assert!(!(*tag_directive).handle.is_null());
                yaml_assert!(!(*tag_directive).prefix.is_null());
                if yaml_check_utf8(
                    (*tag_directive).handle,
                    strlen((*tag_directive).handle as *const c_char),
                ) == 0
                {
                    break 'error;
                }
                if yaml_check_utf8(
                    (*tag_directive).prefix,
                    strlen((*tag_directive).prefix as *const c_char),
                ) == 0
                {
                    break 'error;
                }
                value.handle = crate::memory::yaml_strdup((*tag_directive).handle);
                value.prefix = crate::memory::yaml_strdup((*tag_directive).prefix);
                if value.handle.is_null() || value.prefix.is_null() {
                    break 'error;
                }
                if !crate::PUSH!(context, tag_directives_copy, value) {
                    break 'error;
                }
                value.handle = core::ptr::null_mut();
                value.prefix = core::ptr::null_mut();

                tag_directive = tag_directive.add(1);
            }
        }

        crate::DOCUMENT_START_EVENT_INIT!(
            (*event),
            version_directive_copy,
            tag_directives_copy.start,
            tag_directives_copy.top,
            implicit,
            mark,
            mark
        );

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(version_directive_copy as *mut c_void);
    while !crate::STACK_EMPTY!(context, tag_directives_copy) {
        let value = crate::POP!(context, tag_directives_copy);
        crate::memory::yaml_free(value.handle as *mut c_void);
        crate::memory::yaml_free(value.prefix as *mut c_void);
    }
    crate::STACK_DEL!(context, tag_directives_copy);
    crate::memory::yaml_free(value.handle as *mut c_void);
    crate::memory::yaml_free(value.prefix as *mut c_void);

    return 0;
}

/*
 * Create DOCUMENT-END.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_end_event_initialize(
    event: *mut yaml_event_t,
    implicit: c_int,
) -> c_int {
    let mark = yaml_mark_t::zeroed();

    yaml_assert!(!event.is_null()); /* Non-NULL emitter object is expected. */

    crate::DOCUMENT_END_EVENT_INIT!((*event), implicit, mark, mark);

    return 1;
}

/*
 * Create ALIAS.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_alias_event_initialize(
    event: *mut yaml_event_t,
    anchor: *const yaml_char_t,
) -> c_int {
    let mark = yaml_mark_t::zeroed();
    let anchor_copy: *mut yaml_char_t;

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */
    yaml_assert!(!anchor.is_null()); /* Non-NULL anchor is expected. */

    if yaml_check_utf8(anchor, strlen(anchor as *const c_char)) == 0 {
        return 0;
    }

    anchor_copy = crate::memory::yaml_strdup(anchor);
    if anchor_copy.is_null() {
        return 0;
    }

    crate::ALIAS_EVENT_INIT!((*event), anchor_copy, mark, mark);

    return 1;
}

/*
 * Create SCALAR.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_scalar_event_initialize(
    event: *mut yaml_event_t,
    anchor: *const yaml_char_t,
    tag: *const yaml_char_t,
    value: *const yaml_char_t,
    mut length: c_int,
    plain_implicit: c_int,
    quoted_implicit: c_int,
    style: yaml_scalar_style_t,
) -> c_int {
    let mark = yaml_mark_t::zeroed();
    let mut anchor_copy: *mut yaml_char_t = core::ptr::null_mut();
    let mut tag_copy: *mut yaml_char_t = core::ptr::null_mut();
    let mut value_copy: *mut yaml_char_t = core::ptr::null_mut();

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */
    yaml_assert!(!value.is_null()); /* Non-NULL anchor is expected. */

    'error: {
        if !anchor.is_null() {
            if yaml_check_utf8(anchor, strlen(anchor as *const c_char)) == 0 {
                break 'error;
            }
            anchor_copy = crate::memory::yaml_strdup(anchor);
            if anchor_copy.is_null() {
                break 'error;
            }
        }

        if !tag.is_null() {
            if yaml_check_utf8(tag, strlen(tag as *const c_char)) == 0 {
                break 'error;
            }
            tag_copy = crate::memory::yaml_strdup(tag);
            if tag_copy.is_null() {
                break 'error;
            }
        }

        if length < 0 {
            length = strlen(value as *const c_char) as c_int;
        }

        if yaml_check_utf8(value, length as size_t) == 0 {
            break 'error;
        }
        value_copy = crate::memory::yaml_malloc((length + 1) as size_t) as *mut yaml_char_t;
        if value_copy.is_null() {
            break 'error;
        }
        memcpy(
            value_copy as *mut c_void,
            value as *const c_void,
            length as size_t,
        );
        *value_copy.offset(length as isize) = b'\0';

        crate::SCALAR_EVENT_INIT!(
            (*event),
            anchor_copy,
            tag_copy,
            value_copy,
            length as size_t,
            plain_implicit,
            quoted_implicit,
            style,
            mark,
            mark
        );

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(anchor_copy as *mut c_void);
    crate::memory::yaml_free(tag_copy as *mut c_void);
    crate::memory::yaml_free(value_copy as *mut c_void);

    return 0;
}

/*
 * Create SEQUENCE-START.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_sequence_start_event_initialize(
    event: *mut yaml_event_t,
    anchor: *const yaml_char_t,
    tag: *const yaml_char_t,
    implicit: c_int,
    style: yaml_sequence_style_t,
) -> c_int {
    let mark = yaml_mark_t::zeroed();
    let mut anchor_copy: *mut yaml_char_t = core::ptr::null_mut();
    let mut tag_copy: *mut yaml_char_t = core::ptr::null_mut();

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */

    'error: {
        if !anchor.is_null() {
            if yaml_check_utf8(anchor, strlen(anchor as *const c_char)) == 0 {
                break 'error;
            }
            anchor_copy = crate::memory::yaml_strdup(anchor);
            if anchor_copy.is_null() {
                break 'error;
            }
        }

        if !tag.is_null() {
            if yaml_check_utf8(tag, strlen(tag as *const c_char)) == 0 {
                break 'error;
            }
            tag_copy = crate::memory::yaml_strdup(tag);
            if tag_copy.is_null() {
                break 'error;
            }
        }

        crate::SEQUENCE_START_EVENT_INIT!(
            (*event),
            anchor_copy,
            tag_copy,
            implicit,
            style,
            mark,
            mark
        );

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(anchor_copy as *mut c_void);
    crate::memory::yaml_free(tag_copy as *mut c_void);

    return 0;
}

/*
 * Create SEQUENCE-END.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_sequence_end_event_initialize(event: *mut yaml_event_t) -> c_int {
    let mark = yaml_mark_t::zeroed();

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */

    crate::SEQUENCE_END_EVENT_INIT!((*event), mark, mark);

    return 1;
}

/*
 * Create MAPPING-START.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_mapping_start_event_initialize(
    event: *mut yaml_event_t,
    anchor: *const yaml_char_t,
    tag: *const yaml_char_t,
    implicit: c_int,
    style: yaml_mapping_style_t,
) -> c_int {
    let mark = yaml_mark_t::zeroed();
    let mut anchor_copy: *mut yaml_char_t = core::ptr::null_mut();
    let mut tag_copy: *mut yaml_char_t = core::ptr::null_mut();

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */

    'error: {
        if !anchor.is_null() {
            if yaml_check_utf8(anchor, strlen(anchor as *const c_char)) == 0 {
                break 'error;
            }
            anchor_copy = crate::memory::yaml_strdup(anchor);
            if anchor_copy.is_null() {
                break 'error;
            }
        }

        if !tag.is_null() {
            if yaml_check_utf8(tag, strlen(tag as *const c_char)) == 0 {
                break 'error;
            }
            tag_copy = crate::memory::yaml_strdup(tag);
            if tag_copy.is_null() {
                break 'error;
            }
        }

        crate::MAPPING_START_EVENT_INIT!(
            (*event),
            anchor_copy,
            tag_copy,
            implicit,
            style,
            mark,
            mark
        );

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(anchor_copy as *mut c_void);
    crate::memory::yaml_free(tag_copy as *mut c_void);

    return 0;
}

/*
 * Create MAPPING-END.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_mapping_end_event_initialize(event: *mut yaml_event_t) -> c_int {
    let mark = yaml_mark_t::zeroed();

    yaml_assert!(!event.is_null()); /* Non-NULL event object is expected. */

    crate::MAPPING_END_EVENT_INIT!((*event), mark, mark);

    return 1;
}

/*
 * Destroy an event object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_event_delete(event: *mut yaml_event_t) {
    let mut tag_directive: *mut yaml_tag_directive_t;

    yaml_assert!(!event.is_null()); /* Non-NULL event object expected. */

    match (*event).type_ {
        YAML_DOCUMENT_START_EVENT => {
            crate::memory::yaml_free(
                (*event).data.document_start.version_directive as *mut c_void,
            );
            tag_directive = (*event).data.document_start.tag_directives.start;
            while tag_directive != (*event).data.document_start.tag_directives.end {
                crate::memory::yaml_free((*tag_directive).handle as *mut c_void);
                crate::memory::yaml_free((*tag_directive).prefix as *mut c_void);
                tag_directive = tag_directive.add(1);
            }
            crate::memory::yaml_free(
                (*event).data.document_start.tag_directives.start as *mut c_void,
            );
        }

        YAML_ALIAS_EVENT => {
            crate::memory::yaml_free((*event).data.alias.anchor as *mut c_void);
        }

        YAML_SCALAR_EVENT => {
            crate::memory::yaml_free((*event).data.scalar.anchor as *mut c_void);
            crate::memory::yaml_free((*event).data.scalar.tag as *mut c_void);
            crate::memory::yaml_free((*event).data.scalar.value as *mut c_void);
        }

        YAML_SEQUENCE_START_EVENT => {
            crate::memory::yaml_free((*event).data.sequence_start.anchor as *mut c_void);
            crate::memory::yaml_free((*event).data.sequence_start.tag as *mut c_void);
        }

        YAML_MAPPING_START_EVENT => {
            crate::memory::yaml_free((*event).data.mapping_start.anchor as *mut c_void);
            crate::memory::yaml_free((*event).data.mapping_start.tag as *mut c_void);
        }

        _ => {}
    }

    memset(
        event as *mut c_void,
        0,
        core::mem::size_of::<yaml_event_t>() as size_t,
    );
}
