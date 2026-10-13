//! Translation of `c_src/src/emitter.c` lines 1109-1775: the check_*,
//! process_* and analyze_* helpers.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Check if the document content is an empty scalar.
 */

#[allow(unused_variables)]
pub(crate) unsafe fn yaml_emitter_check_empty_document(emitter: *mut yaml_emitter_t) -> c_int {
    return 0;
}

/*
 * Check if the next events represent an empty sequence.
 */

pub(crate) unsafe fn yaml_emitter_check_empty_sequence(emitter: *mut yaml_emitter_t) -> c_int {
    if (*emitter).events.tail.offset_from((*emitter).events.head) < 2 {
        return 0;
    }

    return ((*(*emitter).events.head.add(0)).type_ == YAML_SEQUENCE_START_EVENT
        && (*(*emitter).events.head.add(1)).type_ == YAML_SEQUENCE_END_EVENT) as c_int;
}

/*
 * Check if the next events represent an empty mapping.
 */

pub(crate) unsafe fn yaml_emitter_check_empty_mapping(emitter: *mut yaml_emitter_t) -> c_int {
    if (*emitter).events.tail.offset_from((*emitter).events.head) < 2 {
        return 0;
    }

    return ((*(*emitter).events.head.add(0)).type_ == YAML_MAPPING_START_EVENT
        && (*(*emitter).events.head.add(1)).type_ == YAML_MAPPING_END_EVENT) as c_int;
}

/*
 * Check if the next node can be expressed as a simple key.
 */

pub(crate) unsafe fn yaml_emitter_check_simple_key(emitter: *mut yaml_emitter_t) -> c_int {
    let event: *mut yaml_event_t = (*emitter).events.head;
    let mut length: size_t = 0;

    match (*event).type_ {
        YAML_ALIAS_EVENT => {
            length += (*emitter).anchor_data.anchor_length;
        }

        YAML_SCALAR_EVENT => {
            if (*emitter).scalar_data.multiline != 0 {
                return 0;
            }
            length += (*emitter).anchor_data.anchor_length
                + (*emitter).tag_data.handle_length
                + (*emitter).tag_data.suffix_length
                + (*emitter).scalar_data.length;
        }

        YAML_SEQUENCE_START_EVENT => {
            if yaml_emitter_check_empty_sequence(emitter) == 0 {
                return 0;
            }
            length += (*emitter).anchor_data.anchor_length
                + (*emitter).tag_data.handle_length
                + (*emitter).tag_data.suffix_length;
        }

        YAML_MAPPING_START_EVENT => {
            if yaml_emitter_check_empty_mapping(emitter) == 0 {
                return 0;
            }
            length += (*emitter).anchor_data.anchor_length
                + (*emitter).tag_data.handle_length
                + (*emitter).tag_data.suffix_length;
        }

        _ => {
            return 0;
        }
    }

    if length > 128 {
        return 0;
    }

    return 1;
}

/*
 * Determine an acceptable scalar style.
 */

pub(crate) unsafe fn yaml_emitter_select_scalar_style(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    let mut style: yaml_scalar_style_t = (*event).data.scalar.style;
    let no_tag: c_int =
        ((*emitter).tag_data.handle.is_null() && (*emitter).tag_data.suffix.is_null()) as c_int;

    if no_tag != 0
        && (*event).data.scalar.plain_implicit == 0
        && (*event).data.scalar.quoted_implicit == 0
    {
        return crate::emitter::yaml_emitter_set_emitter_error(
            emitter,
            b"neither tag nor implicit flags are specified\0".as_ptr() as *const c_char,
        );
    }

    if style == YAML_ANY_SCALAR_STYLE {
        style = YAML_PLAIN_SCALAR_STYLE;
    }

    if (*emitter).canonical != 0 {
        style = YAML_DOUBLE_QUOTED_SCALAR_STYLE;
    }

    if (*emitter).simple_key_context != 0 && (*emitter).scalar_data.multiline != 0 {
        style = YAML_DOUBLE_QUOTED_SCALAR_STYLE;
    }

    if style == YAML_PLAIN_SCALAR_STYLE {
        if ((*emitter).flow_level != 0 && (*emitter).scalar_data.flow_plain_allowed == 0)
            || ((*emitter).flow_level == 0 && (*emitter).scalar_data.block_plain_allowed == 0)
        {
            style = YAML_SINGLE_QUOTED_SCALAR_STYLE;
        }
        if (*emitter).scalar_data.length == 0
            && ((*emitter).flow_level != 0 || (*emitter).simple_key_context != 0)
        {
            style = YAML_SINGLE_QUOTED_SCALAR_STYLE;
        }
        if no_tag != 0 && (*event).data.scalar.plain_implicit == 0 {
            style = YAML_SINGLE_QUOTED_SCALAR_STYLE;
        }
    }

    if style == YAML_SINGLE_QUOTED_SCALAR_STYLE {
        if (*emitter).scalar_data.single_quoted_allowed == 0 {
            style = YAML_DOUBLE_QUOTED_SCALAR_STYLE;
        }
    }

    if style == YAML_LITERAL_SCALAR_STYLE || style == YAML_FOLDED_SCALAR_STYLE {
        if (*emitter).scalar_data.block_allowed == 0
            || (*emitter).flow_level != 0
            || (*emitter).simple_key_context != 0
        {
            style = YAML_DOUBLE_QUOTED_SCALAR_STYLE;
        }
    }

    if no_tag != 0
        && (*event).data.scalar.quoted_implicit == 0
        && style != YAML_PLAIN_SCALAR_STYLE
    {
        (*emitter).tag_data.handle = b"!\0".as_ptr() as *mut yaml_char_t;
        (*emitter).tag_data.handle_length = 1;
    }

    (*emitter).scalar_data.style = style;

    return 1;
}

/*
 * Write an anchor.
 */

pub(crate) unsafe fn yaml_emitter_process_anchor(emitter: *mut yaml_emitter_t) -> c_int {
    if (*emitter).anchor_data.anchor.is_null() {
        return 1;
    }

    if crate::emitter_write::yaml_emitter_write_indicator(
        emitter,
        if (*emitter).anchor_data.alias != 0 {
            b"*\0".as_ptr() as *const c_char
        } else {
            b"&\0".as_ptr() as *const c_char
        },
        1,
        0,
        0,
    ) == 0
    {
        return 0;
    }

    return crate::emitter_write::yaml_emitter_write_anchor(
        emitter,
        (*emitter).anchor_data.anchor,
        (*emitter).anchor_data.anchor_length,
    );
}

/*
 * Write a tag.
 */

pub(crate) unsafe fn yaml_emitter_process_tag(emitter: *mut yaml_emitter_t) -> c_int {
    if (*emitter).tag_data.handle.is_null() && (*emitter).tag_data.suffix.is_null() {
        return 1;
    }

    if !(*emitter).tag_data.handle.is_null() {
        if crate::emitter_write::yaml_emitter_write_tag_handle(
            emitter,
            (*emitter).tag_data.handle,
            (*emitter).tag_data.handle_length,
        ) == 0
        {
            return 0;
        }
        if !(*emitter).tag_data.suffix.is_null() {
            if crate::emitter_write::yaml_emitter_write_tag_content(
                emitter,
                (*emitter).tag_data.suffix,
                (*emitter).tag_data.suffix_length,
                0,
            ) == 0
            {
                return 0;
            }
        }
    } else {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b"!<\0".as_ptr() as *const c_char,
            1,
            0,
            0,
        ) == 0
        {
            return 0;
        }
        if crate::emitter_write::yaml_emitter_write_tag_content(
            emitter,
            (*emitter).tag_data.suffix,
            (*emitter).tag_data.suffix_length,
            0,
        ) == 0
        {
            return 0;
        }
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b">\0".as_ptr() as *const c_char,
            0,
            0,
            0,
        ) == 0
        {
            return 0;
        }
    }

    return 1;
}

/*
 * Write a scalar.
 */

pub(crate) unsafe fn yaml_emitter_process_scalar(emitter: *mut yaml_emitter_t) -> c_int {
    match (*emitter).scalar_data.style {
        YAML_PLAIN_SCALAR_STYLE => {
            return crate::emitter_write::yaml_emitter_write_plain_scalar(
                emitter,
                (*emitter).scalar_data.value,
                (*emitter).scalar_data.length,
                ((*emitter).simple_key_context == 0) as c_int,
            );
        }

        YAML_SINGLE_QUOTED_SCALAR_STYLE => {
            return crate::emitter_write::yaml_emitter_write_single_quoted_scalar(
                emitter,
                (*emitter).scalar_data.value,
                (*emitter).scalar_data.length,
                ((*emitter).simple_key_context == 0) as c_int,
            );
        }

        YAML_DOUBLE_QUOTED_SCALAR_STYLE => {
            return crate::emitter_write::yaml_emitter_write_double_quoted_scalar(
                emitter,
                (*emitter).scalar_data.value,
                (*emitter).scalar_data.length,
                ((*emitter).simple_key_context == 0) as c_int,
            );
        }

        YAML_LITERAL_SCALAR_STYLE => {
            return crate::emitter_write::yaml_emitter_write_literal_scalar(
                emitter,
                (*emitter).scalar_data.value,
                (*emitter).scalar_data.length,
            );
        }

        YAML_FOLDED_SCALAR_STYLE => {
            return crate::emitter_write::yaml_emitter_write_folded_scalar(
                emitter,
                (*emitter).scalar_data.value,
                (*emitter).scalar_data.length,
            );
        }

        _ => {
            yaml_assert!(true); /* Impossible. */
        }
    }

    return 0;
}

/*
 * Check if a %YAML directive is valid.
 */

pub(crate) unsafe fn yaml_emitter_analyze_version_directive(
    emitter: *mut yaml_emitter_t,
    version_directive: yaml_version_directive_t,
) -> c_int {
    if version_directive.major != 1
        || (version_directive.minor != 1 && version_directive.minor != 2)
    {
        return crate::emitter::yaml_emitter_set_emitter_error(
            emitter,
            b"incompatible %YAML directive\0".as_ptr() as *const c_char,
        );
    }

    return 1;
}

/*
 * Check if a %TAG directive is valid.
 */

pub(crate) unsafe fn yaml_emitter_analyze_tag_directive(
    emitter: *mut yaml_emitter_t,
    tag_directive: yaml_tag_directive_t,
) -> c_int {
    let mut handle: yaml_string_t = crate::NULL_STRING!();
    let mut prefix: yaml_string_t = crate::NULL_STRING!();
    let handle_length: size_t;
    let prefix_length: size_t;

    handle_length = strlen(tag_directive.handle as *mut c_char);
    prefix_length = strlen(tag_directive.prefix as *mut c_char);
    crate::STRING_ASSIGN!(handle, tag_directive.handle, handle_length);
    crate::STRING_ASSIGN!(prefix, tag_directive.prefix, prefix_length);

    if handle.start == handle.end {
        return crate::emitter::yaml_emitter_set_emitter_error(
            emitter,
            b"tag handle must not be empty\0".as_ptr() as *const c_char,
        );
    }

    if *handle.start.add(0) != b'!' {
        return crate::emitter::yaml_emitter_set_emitter_error(
            emitter,
            b"tag handle must start with '!'\0".as_ptr() as *const c_char,
        );
    }

    if *handle.end.sub(1) != b'!' {
        return crate::emitter::yaml_emitter_set_emitter_error(
            emitter,
            b"tag handle must end with '!'\0".as_ptr() as *const c_char,
        );
    }

    handle.pointer = handle.pointer.add(1);

    while handle.pointer < handle.end.sub(1) {
        if !crate::IS_ALPHA!(handle) {
            return crate::emitter::yaml_emitter_set_emitter_error(
                emitter,
                b"tag handle must contain alphanumerical characters only\0".as_ptr()
                    as *const c_char,
            );
        }
        crate::MOVE!(handle);
    }

    if prefix.start == prefix.end {
        return crate::emitter::yaml_emitter_set_emitter_error(
            emitter,
            b"tag prefix must not be empty\0".as_ptr() as *const c_char,
        );
    }

    return 1;
}

/*
 * Check if an anchor is valid.
 */

pub(crate) unsafe fn yaml_emitter_analyze_anchor(
    emitter: *mut yaml_emitter_t,
    anchor: *mut yaml_char_t,
    alias: c_int,
) -> c_int {
    let anchor_length: size_t;
    let mut string: yaml_string_t = crate::NULL_STRING!();

    anchor_length = strlen(anchor as *mut c_char);
    crate::STRING_ASSIGN!(string, anchor, anchor_length);

    if string.start == string.end {
        return crate::emitter::yaml_emitter_set_emitter_error(
            emitter,
            if alias != 0 {
                b"alias value must not be empty\0".as_ptr() as *const c_char
            } else {
                b"anchor value must not be empty\0".as_ptr() as *const c_char
            },
        );
    }

    while string.pointer != string.end {
        if !crate::IS_ALPHA!(string) {
            return crate::emitter::yaml_emitter_set_emitter_error(
                emitter,
                if alias != 0 {
                    b"alias value must contain alphanumerical characters only\0".as_ptr()
                        as *const c_char
                } else {
                    b"anchor value must contain alphanumerical characters only\0".as_ptr()
                        as *const c_char
                },
            );
        }
        crate::MOVE!(string);
    }

    (*emitter).anchor_data.anchor = string.start;
    (*emitter).anchor_data.anchor_length = string.end.offset_from(string.start) as size_t;
    (*emitter).anchor_data.alias = alias;

    return 1;
}

/*
 * Check if a tag is valid.
 */

pub(crate) unsafe fn yaml_emitter_analyze_tag(
    emitter: *mut yaml_emitter_t,
    tag: *mut yaml_char_t,
) -> c_int {
    let tag_length: size_t;
    let mut string: yaml_string_t = crate::NULL_STRING!();
    let mut tag_directive: *mut yaml_tag_directive_t;

    tag_length = strlen(tag as *mut c_char);
    crate::STRING_ASSIGN!(string, tag, tag_length);

    if string.start == string.end {
        return crate::emitter::yaml_emitter_set_emitter_error(
            emitter,
            b"tag value must not be empty\0".as_ptr() as *const c_char,
        );
    }

    tag_directive = (*emitter).tag_directives.start;
    while tag_directive != (*emitter).tag_directives.top {
        let prefix_length: size_t = strlen((*tag_directive).prefix as *mut c_char);
        if prefix_length < string.end.offset_from(string.start) as size_t
            && strncmp(
                (*tag_directive).prefix as *mut c_char,
                string.start as *mut c_char,
                prefix_length,
            ) == 0
        {
            (*emitter).tag_data.handle = (*tag_directive).handle;
            (*emitter).tag_data.handle_length = strlen((*tag_directive).handle as *mut c_char);
            (*emitter).tag_data.suffix = string.start.add(prefix_length);
            (*emitter).tag_data.suffix_length =
                (string.end.offset_from(string.start) as size_t) - prefix_length;
            return 1;
        }
        tag_directive = tag_directive.add(1);
    }

    (*emitter).tag_data.suffix = string.start;
    (*emitter).tag_data.suffix_length = string.end.offset_from(string.start) as size_t;

    return 1;
}

/*
 * Check if a scalar is valid.
 */

pub(crate) unsafe fn yaml_emitter_analyze_scalar(
    emitter: *mut yaml_emitter_t,
    value: *mut yaml_char_t,
    length: size_t,
) -> c_int {
    let mut string: yaml_string_t = crate::NULL_STRING!();

    let mut block_indicators: c_int = 0;
    let mut flow_indicators: c_int = 0;
    let mut line_breaks: c_int = 0;
    let mut special_characters: c_int = 0;

    let mut leading_space: c_int = 0;
    let mut leading_break: c_int = 0;
    let mut trailing_space: c_int = 0;
    let mut trailing_break: c_int = 0;
    let mut break_space: c_int = 0;
    let mut space_break: c_int = 0;

    let mut preceded_by_whitespace: c_int = 0;
    let mut followed_by_whitespace: c_int = 0;
    let mut previous_space: c_int = 0;
    let mut previous_break: c_int = 0;

    crate::STRING_ASSIGN!(string, value, length);

    (*emitter).scalar_data.value = value;
    (*emitter).scalar_data.length = length;

    if string.start == string.end {
        (*emitter).scalar_data.multiline = 0;
        (*emitter).scalar_data.flow_plain_allowed = 0;
        (*emitter).scalar_data.block_plain_allowed = 1;
        (*emitter).scalar_data.single_quoted_allowed = 1;
        (*emitter).scalar_data.block_allowed = 0;

        return 1;
    }

    if (crate::CHECK_AT!(string, b'-', 0)
        && crate::CHECK_AT!(string, b'-', 1)
        && crate::CHECK_AT!(string, b'-', 2))
        || (crate::CHECK_AT!(string, b'.', 0)
            && crate::CHECK_AT!(string, b'.', 1)
            && crate::CHECK_AT!(string, b'.', 2))
    {
        block_indicators = 1;
        flow_indicators = 1;
    }

    preceded_by_whitespace = 1;
    followed_by_whitespace = crate::IS_BLANKZ_AT!(string, crate::WIDTH!(string)) as c_int;

    while string.pointer != string.end {
        if string.start == string.pointer {
            if crate::CHECK!(string, b'#')
                || crate::CHECK!(string, b',')
                || crate::CHECK!(string, b'[')
                || crate::CHECK!(string, b']')
                || crate::CHECK!(string, b'{')
                || crate::CHECK!(string, b'}')
                || crate::CHECK!(string, b'&')
                || crate::CHECK!(string, b'*')
                || crate::CHECK!(string, b'!')
                || crate::CHECK!(string, b'|')
                || crate::CHECK!(string, b'>')
                || crate::CHECK!(string, b'\'')
                || crate::CHECK!(string, b'"')
                || crate::CHECK!(string, b'%')
                || crate::CHECK!(string, b'@')
                || crate::CHECK!(string, b'`')
            {
                flow_indicators = 1;
                block_indicators = 1;
            }

            if crate::CHECK!(string, b'?') || crate::CHECK!(string, b':') {
                flow_indicators = 1;
                if followed_by_whitespace != 0 {
                    block_indicators = 1;
                }
            }

            if crate::CHECK!(string, b'-') && followed_by_whitespace != 0 {
                flow_indicators = 1;
                block_indicators = 1;
            }
        } else {
            if crate::CHECK!(string, b',')
                || crate::CHECK!(string, b'?')
                || crate::CHECK!(string, b'[')
                || crate::CHECK!(string, b']')
                || crate::CHECK!(string, b'{')
                || crate::CHECK!(string, b'}')
            {
                flow_indicators = 1;
            }

            if crate::CHECK!(string, b':') {
                flow_indicators = 1;
                if followed_by_whitespace != 0 {
                    block_indicators = 1;
                }
            }

            if crate::CHECK!(string, b'#') && preceded_by_whitespace != 0 {
                flow_indicators = 1;
                block_indicators = 1;
            }
        }

        if !crate::IS_PRINTABLE!(string)
            || (!crate::IS_ASCII!(string) && (*emitter).unicode == 0)
        {
            special_characters = 1;
        }

        if crate::IS_BREAK!(string) {
            line_breaks = 1;
        }

        if crate::IS_SPACE!(string) {
            if string.start == string.pointer {
                leading_space = 1;
            }
            if string.pointer.add(crate::WIDTH!(string) as usize) == string.end {
                trailing_space = 1;
            }
            if previous_break != 0 {
                break_space = 1;
            }
            previous_space = 1;
            previous_break = 0;
        } else if crate::IS_BREAK!(string) {
            if string.start == string.pointer {
                leading_break = 1;
            }
            if string.pointer.add(crate::WIDTH!(string) as usize) == string.end {
                trailing_break = 1;
            }
            if previous_space != 0 {
                space_break = 1;
            }
            previous_space = 0;
            previous_break = 1;
        } else {
            previous_space = 0;
            previous_break = 0;
        }

        preceded_by_whitespace = crate::IS_BLANKZ!(string) as c_int;
        crate::MOVE!(string);
        if string.pointer != string.end {
            followed_by_whitespace = crate::IS_BLANKZ_AT!(string, crate::WIDTH!(string)) as c_int;
        }
    }

    (*emitter).scalar_data.multiline = line_breaks;

    (*emitter).scalar_data.flow_plain_allowed = 1;
    (*emitter).scalar_data.block_plain_allowed = 1;
    (*emitter).scalar_data.single_quoted_allowed = 1;
    (*emitter).scalar_data.block_allowed = 1;

    if leading_space != 0 || leading_break != 0 || trailing_space != 0 || trailing_break != 0 {
        (*emitter).scalar_data.flow_plain_allowed = 0;
        (*emitter).scalar_data.block_plain_allowed = 0;
    }

    if trailing_space != 0 {
        (*emitter).scalar_data.block_allowed = 0;
    }

    if break_space != 0 {
        (*emitter).scalar_data.flow_plain_allowed = 0;
        (*emitter).scalar_data.block_plain_allowed = 0;
        (*emitter).scalar_data.single_quoted_allowed = 0;
    }

    if space_break != 0 || special_characters != 0 {
        (*emitter).scalar_data.flow_plain_allowed = 0;
        (*emitter).scalar_data.block_plain_allowed = 0;
        (*emitter).scalar_data.single_quoted_allowed = 0;
        (*emitter).scalar_data.block_allowed = 0;
    }

    if line_breaks != 0 {
        (*emitter).scalar_data.flow_plain_allowed = 0;
        (*emitter).scalar_data.block_plain_allowed = 0;
    }

    if flow_indicators != 0 {
        (*emitter).scalar_data.flow_plain_allowed = 0;
    }

    if block_indicators != 0 {
        (*emitter).scalar_data.block_plain_allowed = 0;
    }

    return 1;
}

/*
 * Check if the event data is valid.
 */

pub(crate) unsafe fn yaml_emitter_analyze_event(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    (*emitter).anchor_data.anchor = core::ptr::null_mut();
    (*emitter).anchor_data.anchor_length = 0;
    (*emitter).tag_data.handle = core::ptr::null_mut();
    (*emitter).tag_data.handle_length = 0;
    (*emitter).tag_data.suffix = core::ptr::null_mut();
    (*emitter).tag_data.suffix_length = 0;
    (*emitter).scalar_data.value = core::ptr::null_mut();
    (*emitter).scalar_data.length = 0;

    match (*event).type_ {
        YAML_ALIAS_EVENT => {
            if yaml_emitter_analyze_anchor(emitter, (*event).data.alias.anchor, 1) == 0 {
                return 0;
            }
            return 1;
        }

        YAML_SCALAR_EVENT => {
            if !(*event).data.scalar.anchor.is_null() {
                if yaml_emitter_analyze_anchor(emitter, (*event).data.scalar.anchor, 0) == 0 {
                    return 0;
                }
            }
            if !(*event).data.scalar.tag.is_null()
                && ((*emitter).canonical != 0
                    || ((*event).data.scalar.plain_implicit == 0
                        && (*event).data.scalar.quoted_implicit == 0))
            {
                if yaml_emitter_analyze_tag(emitter, (*event).data.scalar.tag) == 0 {
                    return 0;
                }
            }
            if yaml_emitter_analyze_scalar(
                emitter,
                (*event).data.scalar.value,
                (*event).data.scalar.length,
            ) == 0
            {
                return 0;
            }
            return 1;
        }

        YAML_SEQUENCE_START_EVENT => {
            if !(*event).data.sequence_start.anchor.is_null() {
                if yaml_emitter_analyze_anchor(emitter, (*event).data.sequence_start.anchor, 0) == 0
                {
                    return 0;
                }
            }
            if !(*event).data.sequence_start.tag.is_null()
                && ((*emitter).canonical != 0 || (*event).data.sequence_start.implicit == 0)
            {
                if yaml_emitter_analyze_tag(emitter, (*event).data.sequence_start.tag) == 0 {
                    return 0;
                }
            }
            return 1;
        }

        YAML_MAPPING_START_EVENT => {
            if !(*event).data.mapping_start.anchor.is_null() {
                if yaml_emitter_analyze_anchor(emitter, (*event).data.mapping_start.anchor, 0) == 0 {
                    return 0;
                }
            }
            if !(*event).data.mapping_start.tag.is_null()
                && ((*emitter).canonical != 0 || (*event).data.mapping_start.implicit == 0)
            {
                if yaml_emitter_analyze_tag(emitter, (*event).data.mapping_start.tag) == 0 {
                    return 0;
                }
            }
            return 1;
        }

        _ => {
            return 1;
        }
    }
}
