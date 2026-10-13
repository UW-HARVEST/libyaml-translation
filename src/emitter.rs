//! Translation of `c_src/src/emitter.c` lines 264-1108: the emitter entry
//! point and the event state machine.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Set an emitter error and return 0.
 */

pub(crate) unsafe fn yaml_emitter_set_emitter_error(
    emitter: *mut yaml_emitter_t,
    problem: *const c_char,
) -> c_int {
    (*emitter).error = YAML_EMITTER_ERROR;
    (*emitter).problem = problem;

    return 0;
}

/*
 * Emit an event.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_emit(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    if !crate::ENQUEUE!(emitter, (*emitter).events, *event) {
        crate::api_objects::yaml_event_delete(event);
        return 0;
    }

    while yaml_emitter_need_more_events(emitter) == 0 {
        if crate::emitter_analyze::yaml_emitter_analyze_event(emitter, (*emitter).events.head) == 0
        {
            return 0;
        }
        if yaml_emitter_state_machine(emitter, (*emitter).events.head) == 0 {
            return 0;
        }
        let mut __event = crate::DEQUEUE!(emitter, (*emitter).events);
        crate::api_objects::yaml_event_delete(&raw mut __event);
    }

    return 1;
}

/*
 * Check if we need to accumulate more events before emitting.
 *
 * We accumulate extra
 *  - 1 event for DOCUMENT-START
 *  - 2 events for SEQUENCE-START
 *  - 3 events for MAPPING-START
 */

pub(crate) unsafe fn yaml_emitter_need_more_events(emitter: *mut yaml_emitter_t) -> c_int {
    let mut level: c_int = 0;
    let accumulate: c_int;
    let mut event: *mut yaml_event_t;

    if crate::QUEUE_EMPTY!(emitter, (*emitter).events) {
        return 1;
    }

    match (*(*emitter).events.head).type_ {
        YAML_DOCUMENT_START_EVENT => {
            accumulate = 1;
        }
        YAML_SEQUENCE_START_EVENT => {
            accumulate = 2;
        }
        YAML_MAPPING_START_EVENT => {
            accumulate = 3;
        }
        _ => {
            return 0;
        }
    }

    if (*emitter).events.tail.offset_from((*emitter).events.head) > accumulate as isize {
        return 0;
    }

    event = (*emitter).events.head;
    while event != (*emitter).events.tail {
        match (*event).type_ {
            YAML_STREAM_START_EVENT
            | YAML_DOCUMENT_START_EVENT
            | YAML_SEQUENCE_START_EVENT
            | YAML_MAPPING_START_EVENT => {
                level += 1;
            }
            YAML_STREAM_END_EVENT
            | YAML_DOCUMENT_END_EVENT
            | YAML_SEQUENCE_END_EVENT
            | YAML_MAPPING_END_EVENT => {
                level -= 1;
            }
            _ => {}
        }
        if level == 0 {
            return 0;
        }
        event = event.add(1);
    }

    return 1;
}

/*
 * Append a directive to the directives stack.
 */

pub(crate) unsafe fn yaml_emitter_append_tag_directive(
    emitter: *mut yaml_emitter_t,
    value: yaml_tag_directive_t,
    allow_duplicates: c_int,
) -> c_int {
    let mut tag_directive: *mut yaml_tag_directive_t;
    let mut copy: yaml_tag_directive_t = yaml_tag_directive_t {
        handle: core::ptr::null_mut(),
        prefix: core::ptr::null_mut(),
    };

    'error: {
        tag_directive = (*emitter).tag_directives.start;
        while tag_directive != (*emitter).tag_directives.top {
            if strcmp(
                value.handle as *const c_char,
                (*tag_directive).handle as *const c_char,
            ) == 0
            {
                if allow_duplicates != 0 {
                    return 1;
                }
                return yaml_emitter_set_emitter_error(
                    emitter,
                    b"duplicate %TAG directive\0".as_ptr() as *const c_char,
                );
            }
            tag_directive = tag_directive.add(1);
        }

        copy.handle = crate::memory::yaml_strdup(value.handle);
        copy.prefix = crate::memory::yaml_strdup(value.prefix);
        if copy.handle.is_null() || copy.prefix.is_null() {
            (*emitter).error = YAML_MEMORY_ERROR;
            break 'error;
        }

        if !crate::PUSH!(emitter, (*emitter).tag_directives, copy) {
            break 'error;
        }

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(copy.handle as *mut c_void);
    crate::memory::yaml_free(copy.prefix as *mut c_void);
    return 0;
}

/*
 * Increase the indentation level.
 */

pub(crate) unsafe fn yaml_emitter_increase_indent(
    emitter: *mut yaml_emitter_t,
    flow: c_int,
    indentless: c_int,
) -> c_int {
    if !crate::PUSH!(emitter, (*emitter).indents, (*emitter).indent) {
        return 0;
    }

    if (*emitter).indent < 0 {
        (*emitter).indent = if flow != 0 { (*emitter).best_indent } else { 0 };
    } else if indentless == 0 {
        (*emitter).indent += (*emitter).best_indent;
    }

    return 1;
}

/*
 * State dispatcher.
 */

pub(crate) unsafe fn yaml_emitter_state_machine(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    match (*emitter).state {
        YAML_EMIT_STREAM_START_STATE => {
            return yaml_emitter_emit_stream_start(emitter, event);
        }

        YAML_EMIT_FIRST_DOCUMENT_START_STATE => {
            return yaml_emitter_emit_document_start(emitter, event, 1);
        }

        YAML_EMIT_DOCUMENT_START_STATE => {
            return yaml_emitter_emit_document_start(emitter, event, 0);
        }

        YAML_EMIT_DOCUMENT_CONTENT_STATE => {
            return yaml_emitter_emit_document_content(emitter, event);
        }

        YAML_EMIT_DOCUMENT_END_STATE => {
            return yaml_emitter_emit_document_end(emitter, event);
        }

        YAML_EMIT_FLOW_SEQUENCE_FIRST_ITEM_STATE => {
            return yaml_emitter_emit_flow_sequence_item(emitter, event, 1);
        }

        YAML_EMIT_FLOW_SEQUENCE_ITEM_STATE => {
            return yaml_emitter_emit_flow_sequence_item(emitter, event, 0);
        }

        YAML_EMIT_FLOW_MAPPING_FIRST_KEY_STATE => {
            return yaml_emitter_emit_flow_mapping_key(emitter, event, 1);
        }

        YAML_EMIT_FLOW_MAPPING_KEY_STATE => {
            return yaml_emitter_emit_flow_mapping_key(emitter, event, 0);
        }

        YAML_EMIT_FLOW_MAPPING_SIMPLE_VALUE_STATE => {
            return yaml_emitter_emit_flow_mapping_value(emitter, event, 1);
        }

        YAML_EMIT_FLOW_MAPPING_VALUE_STATE => {
            return yaml_emitter_emit_flow_mapping_value(emitter, event, 0);
        }

        YAML_EMIT_BLOCK_SEQUENCE_FIRST_ITEM_STATE => {
            return yaml_emitter_emit_block_sequence_item(emitter, event, 1);
        }

        YAML_EMIT_BLOCK_SEQUENCE_ITEM_STATE => {
            return yaml_emitter_emit_block_sequence_item(emitter, event, 0);
        }

        YAML_EMIT_BLOCK_MAPPING_FIRST_KEY_STATE => {
            return yaml_emitter_emit_block_mapping_key(emitter, event, 1);
        }

        YAML_EMIT_BLOCK_MAPPING_KEY_STATE => {
            return yaml_emitter_emit_block_mapping_key(emitter, event, 0);
        }

        YAML_EMIT_BLOCK_MAPPING_SIMPLE_VALUE_STATE => {
            return yaml_emitter_emit_block_mapping_value(emitter, event, 1);
        }

        YAML_EMIT_BLOCK_MAPPING_VALUE_STATE => {
            return yaml_emitter_emit_block_mapping_value(emitter, event, 0);
        }

        YAML_EMIT_END_STATE => {
            return yaml_emitter_set_emitter_error(
                emitter,
                b"expected nothing after STREAM-END\0".as_ptr() as *const c_char,
            );
        }

        _ => {
            /* NOTE: the C source really writes `assert(1)` here, which never
             * fires; reproduced verbatim so the behaviour matches. */
            yaml_assert!(1 != 0); /* Invalid state. */
        }
    }

    return 0;
}

/*
 * Expect STREAM-START.
 */

pub(crate) unsafe fn yaml_emitter_emit_stream_start(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    (*emitter).open_ended = 0;
    if (*event).type_ == YAML_STREAM_START_EVENT {
        if (*emitter).encoding == 0 {
            (*emitter).encoding = (*event).data.stream_start.encoding;
        }

        if (*emitter).encoding == 0 {
            (*emitter).encoding = YAML_UTF8_ENCODING;
        }

        if (*emitter).best_indent < 2 || (*emitter).best_indent > 9 {
            (*emitter).best_indent = 2;
        }

        if (*emitter).best_width >= 0 && (*emitter).best_width <= (*emitter).best_indent * 2 {
            (*emitter).best_width = 80;
        }

        if (*emitter).best_width < 0 {
            (*emitter).best_width = INT_MAX;
        }

        if (*emitter).line_break == 0 {
            (*emitter).line_break = YAML_LN_BREAK;
        }

        (*emitter).indent = -1;

        (*emitter).line = 0;
        (*emitter).column = 0;
        (*emitter).whitespace = 1;
        (*emitter).indention = 1;

        if (*emitter).encoding != YAML_UTF8_ENCODING {
            if crate::emitter_write::yaml_emitter_write_bom(emitter) == 0 {
                return 0;
            }
        }

        (*emitter).state = YAML_EMIT_FIRST_DOCUMENT_START_STATE;

        return 1;
    }

    return yaml_emitter_set_emitter_error(
        emitter,
        b"expected STREAM-START\0".as_ptr() as *const c_char,
    );
}

/*
 * Expect DOCUMENT-START or STREAM-END.
 */

pub(crate) unsafe fn yaml_emitter_emit_document_start(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if (*event).type_ == YAML_DOCUMENT_START_EVENT {
        let mut default_tag_directives: [yaml_tag_directive_t; 3] = [
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
        let mut tag_directive: *mut yaml_tag_directive_t;
        let mut implicit: c_int;

        if !(*event).data.document_start.version_directive.is_null() {
            if crate::emitter_analyze::yaml_emitter_analyze_version_directive(
                emitter,
                *(*event).data.document_start.version_directive,
            ) == 0
            {
                return 0;
            }
        }

        tag_directive = (*event).data.document_start.tag_directives.start;
        while tag_directive != (*event).data.document_start.tag_directives.end {
            if crate::emitter_analyze::yaml_emitter_analyze_tag_directive(emitter, *tag_directive)
                == 0
            {
                return 0;
            }
            if yaml_emitter_append_tag_directive(emitter, *tag_directive, 0) == 0 {
                return 0;
            }
            tag_directive = tag_directive.add(1);
        }

        tag_directive = default_tag_directives.as_mut_ptr();
        while !(*tag_directive).handle.is_null() {
            if yaml_emitter_append_tag_directive(emitter, *tag_directive, 1) == 0 {
                return 0;
            }
            tag_directive = tag_directive.add(1);
        }

        implicit = (*event).data.document_start.implicit;
        if first == 0 || (*emitter).canonical != 0 {
            implicit = 0;
        }

        if (!(*event).data.document_start.version_directive.is_null()
            || ((*event).data.document_start.tag_directives.start
                != (*event).data.document_start.tag_directives.end))
            && (*emitter).open_ended != 0
        {
            if crate::emitter_write::yaml_emitter_write_indicator(
                emitter,
                b"...\0".as_ptr() as *const c_char,
                1,
                0,
                0,
            ) == 0
            {
                return 0;
            }
            if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                return 0;
            }
        }
        (*emitter).open_ended = 0;

        if !(*event).data.document_start.version_directive.is_null() {
            implicit = 0;
            if crate::emitter_write::yaml_emitter_write_indicator(
                emitter,
                b"%YAML\0".as_ptr() as *const c_char,
                1,
                0,
                0,
            ) == 0
            {
                return 0;
            }
            if (*(*event).data.document_start.version_directive).minor == 1 {
                if crate::emitter_write::yaml_emitter_write_indicator(
                    emitter,
                    b"1.1\0".as_ptr() as *const c_char,
                    1,
                    0,
                    0,
                ) == 0
                {
                    return 0;
                }
            } else {
                if crate::emitter_write::yaml_emitter_write_indicator(
                    emitter,
                    b"1.2\0".as_ptr() as *const c_char,
                    1,
                    0,
                    0,
                ) == 0
                {
                    return 0;
                }
            }
            if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                return 0;
            }
        }

        if (*event).data.document_start.tag_directives.start
            != (*event).data.document_start.tag_directives.end
        {
            implicit = 0;
            tag_directive = (*event).data.document_start.tag_directives.start;
            while tag_directive != (*event).data.document_start.tag_directives.end {
                if crate::emitter_write::yaml_emitter_write_indicator(
                    emitter,
                    b"%TAG\0".as_ptr() as *const c_char,
                    1,
                    0,
                    0,
                ) == 0
                {
                    return 0;
                }
                if crate::emitter_write::yaml_emitter_write_tag_handle(
                    emitter,
                    (*tag_directive).handle,
                    strlen((*tag_directive).handle as *const c_char),
                ) == 0
                {
                    return 0;
                }
                if crate::emitter_write::yaml_emitter_write_tag_content(
                    emitter,
                    (*tag_directive).prefix,
                    strlen((*tag_directive).prefix as *const c_char),
                    1,
                ) == 0
                {
                    return 0;
                }
                if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
                tag_directive = tag_directive.add(1);
            }
        }

        if crate::emitter_analyze::yaml_emitter_check_empty_document(emitter) != 0 {
            implicit = 0;
        }

        if implicit == 0 {
            if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                return 0;
            }
            if crate::emitter_write::yaml_emitter_write_indicator(
                emitter,
                b"---\0".as_ptr() as *const c_char,
                1,
                0,
                0,
            ) == 0
            {
                return 0;
            }
            if (*emitter).canonical != 0 {
                if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                    return 0;
                }
            }
        }

        (*emitter).state = YAML_EMIT_DOCUMENT_CONTENT_STATE;

        (*emitter).open_ended = 0;
        return 1;
    } else if (*event).type_ == YAML_STREAM_END_EVENT {
        /*
         * This can happen if a block scalar with trailing empty lines
         * is at the end of the stream
         */
        if (*emitter).open_ended == 2 {
            if crate::emitter_write::yaml_emitter_write_indicator(
                emitter,
                b"...\0".as_ptr() as *const c_char,
                1,
                0,
                0,
            ) == 0
            {
                return 0;
            }
            (*emitter).open_ended = 0;
            if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                return 0;
            }
        }
        if crate::writer::yaml_emitter_flush(emitter) == 0 {
            return 0;
        }

        (*emitter).state = YAML_EMIT_END_STATE;

        return 1;
    }

    return yaml_emitter_set_emitter_error(
        emitter,
        b"expected DOCUMENT-START or STREAM-END\0".as_ptr() as *const c_char,
    );
}

/*
 * Expect the root node.
 */

pub(crate) unsafe fn yaml_emitter_emit_document_content(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    if !crate::PUSH!(emitter, (*emitter).states, YAML_EMIT_DOCUMENT_END_STATE) {
        return 0;
    }

    return yaml_emitter_emit_node(emitter, event, 1, 0, 0, 0);
}

/*
 * Expect DOCUMENT-END.
 */

pub(crate) unsafe fn yaml_emitter_emit_document_end(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    if (*event).type_ == YAML_DOCUMENT_END_EVENT {
        if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
            return 0;
        }
        if (*event).data.document_end.implicit == 0 {
            if crate::emitter_write::yaml_emitter_write_indicator(
                emitter,
                b"...\0".as_ptr() as *const c_char,
                1,
                0,
                0,
            ) == 0
            {
                return 0;
            }
            (*emitter).open_ended = 0;
            if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                return 0;
            }
        } else if (*emitter).open_ended == 0 {
            (*emitter).open_ended = 1;
        }
        if crate::writer::yaml_emitter_flush(emitter) == 0 {
            return 0;
        }

        (*emitter).state = YAML_EMIT_DOCUMENT_START_STATE;

        while !crate::STACK_EMPTY!(emitter, (*emitter).tag_directives) {
            let tag_directive: yaml_tag_directive_t =
                crate::POP!(emitter, (*emitter).tag_directives);
            crate::memory::yaml_free(tag_directive.handle as *mut c_void);
            crate::memory::yaml_free(tag_directive.prefix as *mut c_void);
        }

        return 1;
    }

    return yaml_emitter_set_emitter_error(
        emitter,
        b"expected DOCUMENT-END\0".as_ptr() as *const c_char,
    );
}

/*
 *
 * Expect a flow item node.
 */

pub(crate) unsafe fn yaml_emitter_emit_flow_sequence_item(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if first != 0 {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b"[\0".as_ptr() as *const c_char,
            1,
            1,
            0,
        ) == 0
        {
            return 0;
        }
        if yaml_emitter_increase_indent(emitter, 1, 0) == 0 {
            return 0;
        }
        (*emitter).flow_level += 1;
    }

    if (*event).type_ == YAML_SEQUENCE_END_EVENT {
        (*emitter).flow_level -= 1;
        (*emitter).indent = crate::POP!(emitter, (*emitter).indents);
        if (*emitter).canonical != 0 && first == 0 {
            if crate::emitter_write::yaml_emitter_write_indicator(
                emitter,
                b",\0".as_ptr() as *const c_char,
                0,
                0,
                0,
            ) == 0
            {
                return 0;
            }
            if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                return 0;
            }
        }
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b"]\0".as_ptr() as *const c_char,
            0,
            0,
            0,
        ) == 0
        {
            return 0;
        }
        (*emitter).state = crate::POP!(emitter, (*emitter).states);

        return 1;
    }

    if first == 0 {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b",\0".as_ptr() as *const c_char,
            0,
            0,
            0,
        ) == 0
        {
            return 0;
        }
    }

    if (*emitter).canonical != 0 || (*emitter).column > (*emitter).best_width {
        if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
            return 0;
        }
    }
    if !crate::PUSH!(
        emitter,
        (*emitter).states,
        YAML_EMIT_FLOW_SEQUENCE_ITEM_STATE
    ) {
        return 0;
    }

    return yaml_emitter_emit_node(emitter, event, 0, 1, 0, 0);
}

/*
 * Expect a flow key node.
 */

pub(crate) unsafe fn yaml_emitter_emit_flow_mapping_key(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if first != 0 {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b"{\0".as_ptr() as *const c_char,
            1,
            1,
            0,
        ) == 0
        {
            return 0;
        }
        if yaml_emitter_increase_indent(emitter, 1, 0) == 0 {
            return 0;
        }
        (*emitter).flow_level += 1;
    }

    if (*event).type_ == YAML_MAPPING_END_EVENT {
        (*emitter).flow_level -= 1;
        (*emitter).indent = crate::POP!(emitter, (*emitter).indents);
        if (*emitter).canonical != 0 && first == 0 {
            if crate::emitter_write::yaml_emitter_write_indicator(
                emitter,
                b",\0".as_ptr() as *const c_char,
                0,
                0,
                0,
            ) == 0
            {
                return 0;
            }
            if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                return 0;
            }
        }
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b"}\0".as_ptr() as *const c_char,
            0,
            0,
            0,
        ) == 0
        {
            return 0;
        }
        (*emitter).state = crate::POP!(emitter, (*emitter).states);

        return 1;
    }

    if first == 0 {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b",\0".as_ptr() as *const c_char,
            0,
            0,
            0,
        ) == 0
        {
            return 0;
        }
    }
    if (*emitter).canonical != 0 || (*emitter).column > (*emitter).best_width {
        if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
            return 0;
        }
    }

    if (*emitter).canonical == 0
        && crate::emitter_analyze::yaml_emitter_check_simple_key(emitter) != 0
    {
        if !crate::PUSH!(
            emitter,
            (*emitter).states,
            YAML_EMIT_FLOW_MAPPING_SIMPLE_VALUE_STATE
        ) {
            return 0;
        }

        return yaml_emitter_emit_node(emitter, event, 0, 0, 1, 1);
    } else {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b"?\0".as_ptr() as *const c_char,
            1,
            0,
            0,
        ) == 0
        {
            return 0;
        }
        if !crate::PUSH!(
            emitter,
            (*emitter).states,
            YAML_EMIT_FLOW_MAPPING_VALUE_STATE
        ) {
            return 0;
        }

        return yaml_emitter_emit_node(emitter, event, 0, 0, 1, 0);
    }
}

/*
 * Expect a flow value node.
 */

pub(crate) unsafe fn yaml_emitter_emit_flow_mapping_value(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    simple: c_int,
) -> c_int {
    if simple != 0 {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b":\0".as_ptr() as *const c_char,
            0,
            0,
            0,
        ) == 0
        {
            return 0;
        }
    } else {
        if (*emitter).canonical != 0 || (*emitter).column > (*emitter).best_width {
            if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
                return 0;
            }
        }
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b":\0".as_ptr() as *const c_char,
            1,
            0,
            0,
        ) == 0
        {
            return 0;
        }
    }
    if !crate::PUSH!(
        emitter,
        (*emitter).states,
        YAML_EMIT_FLOW_MAPPING_KEY_STATE
    ) {
        return 0;
    }
    return yaml_emitter_emit_node(emitter, event, 0, 0, 1, 0);
}

/*
 * Expect a block item node.
 */

pub(crate) unsafe fn yaml_emitter_emit_block_sequence_item(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if first != 0 {
        if yaml_emitter_increase_indent(
            emitter,
            0,
            ((*emitter).mapping_context != 0 && (*emitter).indention == 0) as c_int,
        ) == 0
        {
            return 0;
        }
    }

    if (*event).type_ == YAML_SEQUENCE_END_EVENT {
        (*emitter).indent = crate::POP!(emitter, (*emitter).indents);
        (*emitter).state = crate::POP!(emitter, (*emitter).states);

        return 1;
    }

    if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
        return 0;
    }
    if crate::emitter_write::yaml_emitter_write_indicator(
        emitter,
        b"-\0".as_ptr() as *const c_char,
        1,
        0,
        1,
    ) == 0
    {
        return 0;
    }
    if !crate::PUSH!(
        emitter,
        (*emitter).states,
        YAML_EMIT_BLOCK_SEQUENCE_ITEM_STATE
    ) {
        return 0;
    }

    return yaml_emitter_emit_node(emitter, event, 0, 1, 0, 0);
}

/*
 * Expect a block key node.
 */

pub(crate) unsafe fn yaml_emitter_emit_block_mapping_key(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if first != 0 {
        if yaml_emitter_increase_indent(emitter, 0, 0) == 0 {
            return 0;
        }
    }

    if (*event).type_ == YAML_MAPPING_END_EVENT {
        (*emitter).indent = crate::POP!(emitter, (*emitter).indents);
        (*emitter).state = crate::POP!(emitter, (*emitter).states);

        return 1;
    }

    if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
        return 0;
    }

    if crate::emitter_analyze::yaml_emitter_check_simple_key(emitter) != 0 {
        if !crate::PUSH!(
            emitter,
            (*emitter).states,
            YAML_EMIT_BLOCK_MAPPING_SIMPLE_VALUE_STATE
        ) {
            return 0;
        }

        return yaml_emitter_emit_node(emitter, event, 0, 0, 1, 1);
    } else {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b"?\0".as_ptr() as *const c_char,
            1,
            0,
            1,
        ) == 0
        {
            return 0;
        }
        if !crate::PUSH!(
            emitter,
            (*emitter).states,
            YAML_EMIT_BLOCK_MAPPING_VALUE_STATE
        ) {
            return 0;
        }

        return yaml_emitter_emit_node(emitter, event, 0, 0, 1, 0);
    }
}

/*
 * Expect a block value node.
 */

pub(crate) unsafe fn yaml_emitter_emit_block_mapping_value(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    simple: c_int,
) -> c_int {
    if simple != 0 {
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b":\0".as_ptr() as *const c_char,
            0,
            0,
            0,
        ) == 0
        {
            return 0;
        }
    } else {
        if crate::emitter_write::yaml_emitter_write_indent(emitter) == 0 {
            return 0;
        }
        if crate::emitter_write::yaml_emitter_write_indicator(
            emitter,
            b":\0".as_ptr() as *const c_char,
            1,
            0,
            1,
        ) == 0
        {
            return 0;
        }
    }
    if !crate::PUSH!(
        emitter,
        (*emitter).states,
        YAML_EMIT_BLOCK_MAPPING_KEY_STATE
    ) {
        return 0;
    }

    return yaml_emitter_emit_node(emitter, event, 0, 0, 1, 0);
}

/*
 * Expect a node.
 */

#[allow(unreachable_code)]
pub(crate) unsafe fn yaml_emitter_emit_node(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    root: c_int,
    sequence: c_int,
    mapping: c_int,
    simple_key: c_int,
) -> c_int {
    (*emitter).root_context = root;
    (*emitter).sequence_context = sequence;
    (*emitter).mapping_context = mapping;
    (*emitter).simple_key_context = simple_key;

    match (*event).type_ {
        YAML_ALIAS_EVENT => {
            return yaml_emitter_emit_alias(emitter, event);
        }

        YAML_SCALAR_EVENT => {
            return yaml_emitter_emit_scalar(emitter, event);
        }

        YAML_SEQUENCE_START_EVENT => {
            return yaml_emitter_emit_sequence_start(emitter, event);
        }

        YAML_MAPPING_START_EVENT => {
            return yaml_emitter_emit_mapping_start(emitter, event);
        }

        _ => {
            return yaml_emitter_set_emitter_error(
                emitter,
                b"expected SCALAR, SEQUENCE-START, MAPPING-START, or ALIAS\0".as_ptr()
                    as *const c_char,
            );
        }
    }

    return 0;
}

/*
 * Expect ALIAS.
 */

#[allow(unused_variables)]
pub(crate) unsafe fn yaml_emitter_emit_alias(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    if crate::emitter_analyze::yaml_emitter_process_anchor(emitter) == 0 {
        return 0;
    }
    if (*emitter).simple_key_context != 0 {
        if !crate::PUT!(emitter, b' ') {
            return 0;
        }
    }
    (*emitter).state = crate::POP!(emitter, (*emitter).states);

    return 1;
}

/*
 * Expect SCALAR.
 */

pub(crate) unsafe fn yaml_emitter_emit_scalar(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    if crate::emitter_analyze::yaml_emitter_select_scalar_style(emitter, event) == 0 {
        return 0;
    }
    if crate::emitter_analyze::yaml_emitter_process_anchor(emitter) == 0 {
        return 0;
    }
    if crate::emitter_analyze::yaml_emitter_process_tag(emitter) == 0 {
        return 0;
    }
    if yaml_emitter_increase_indent(emitter, 1, 0) == 0 {
        return 0;
    }
    if crate::emitter_analyze::yaml_emitter_process_scalar(emitter) == 0 {
        return 0;
    }
    (*emitter).indent = crate::POP!(emitter, (*emitter).indents);
    (*emitter).state = crate::POP!(emitter, (*emitter).states);

    return 1;
}

/*
 * Expect SEQUENCE-START.
 */

pub(crate) unsafe fn yaml_emitter_emit_sequence_start(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    if crate::emitter_analyze::yaml_emitter_process_anchor(emitter) == 0 {
        return 0;
    }
    if crate::emitter_analyze::yaml_emitter_process_tag(emitter) == 0 {
        return 0;
    }

    if (*emitter).flow_level != 0
        || (*emitter).canonical != 0
        || (*event).data.sequence_start.style == YAML_FLOW_SEQUENCE_STYLE
        || crate::emitter_analyze::yaml_emitter_check_empty_sequence(emitter) != 0
    {
        (*emitter).state = YAML_EMIT_FLOW_SEQUENCE_FIRST_ITEM_STATE;
    } else {
        (*emitter).state = YAML_EMIT_BLOCK_SEQUENCE_FIRST_ITEM_STATE;
    }

    return 1;
}

/*
 * Expect MAPPING-START.
 */

pub(crate) unsafe fn yaml_emitter_emit_mapping_start(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
) -> c_int {
    if crate::emitter_analyze::yaml_emitter_process_anchor(emitter) == 0 {
        return 0;
    }
    if crate::emitter_analyze::yaml_emitter_process_tag(emitter) == 0 {
        return 0;
    }

    if (*emitter).flow_level != 0
        || (*emitter).canonical != 0
        || (*event).data.mapping_start.style == YAML_FLOW_MAPPING_STYLE
        || crate::emitter_analyze::yaml_emitter_check_empty_mapping(emitter) != 0
    {
        (*emitter).state = YAML_EMIT_FLOW_MAPPING_FIRST_KEY_STATE;
    } else {
        (*emitter).state = YAML_EMIT_BLOCK_MAPPING_FIRST_KEY_STATE;
    }

    return 1;
}
