//! Translation of `c_src/src/dumper.c`.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Issue a STREAM-START event.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_open(emitter: *mut yaml_emitter_t) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();
    let mark: yaml_mark_t = yaml_mark_t::zeroed();

    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object is required. */
    yaml_assert!((*emitter).opened == 0); /* Emitter should not be opened yet. */

    crate::STREAM_START_EVENT_INIT!(event, YAML_ANY_ENCODING, mark, mark);

    if crate::emitter::yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    (*emitter).opened = 1;

    return 1;
}

/*
 * Issue a STREAM-END event.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_close(emitter: *mut yaml_emitter_t) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();
    let mark: yaml_mark_t = yaml_mark_t::zeroed();

    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object is required. */
    yaml_assert!((*emitter).opened != 0); /* Emitter should be opened. */

    if (*emitter).closed != 0 {
        return 1;
    }

    crate::STREAM_END_EVENT_INIT!(event, mark, mark);

    if crate::emitter::yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    (*emitter).closed = 1;

    return 1;
}

/*
 * Dump a YAML document.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_dump(
    emitter: *mut yaml_emitter_t,
    document: *mut yaml_document_t,
) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();
    let mark: yaml_mark_t = yaml_mark_t::zeroed();

    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object is required. */
    yaml_assert!(!document.is_null()); /* Non-NULL emitter object is expected. */

    'error: {
        (*emitter).document = document;

        if (*emitter).opened == 0 {
            if yaml_emitter_open(emitter) == 0 {
                break 'error;
            }
        }

        if crate::STACK_EMPTY!(emitter, (*document).nodes) {
            if yaml_emitter_close(emitter) == 0 {
                break 'error;
            }
            yaml_emitter_delete_document_and_anchors(emitter);
            return 1;
        }

        yaml_assert!((*emitter).opened != 0); /* Emitter should be opened. */

        (*emitter).anchors = crate::memory::yaml_malloc(
            core::mem::size_of::<yaml_anchors_t>()
                * ((*document).nodes.top.offset_from((*document).nodes.start) as size_t),
        ) as *mut yaml_anchors_t;
        if (*emitter).anchors.is_null() {
            break 'error;
        }
        memset(
            (*emitter).anchors as *mut c_void,
            0,
            core::mem::size_of::<yaml_anchors_t>()
                * ((*document).nodes.top.offset_from((*document).nodes.start) as size_t),
        );

        crate::DOCUMENT_START_EVENT_INIT!(
            event,
            (*document).version_directive,
            (*document).tag_directives.start,
            (*document).tag_directives.end,
            (*document).start_implicit,
            mark,
            mark
        );
        if crate::emitter::yaml_emitter_emit(emitter, &mut event) == 0 {
            break 'error;
        }

        yaml_emitter_anchor_node(emitter, 1);
        if yaml_emitter_dump_node(emitter, 1) == 0 {
            break 'error;
        }

        crate::DOCUMENT_END_EVENT_INIT!(event, (*document).end_implicit, mark, mark);
        if crate::emitter::yaml_emitter_emit(emitter, &mut event) == 0 {
            break 'error;
        }

        yaml_emitter_delete_document_and_anchors(emitter);

        return 1;
    }

    /* error: */

    yaml_emitter_delete_document_and_anchors(emitter);

    return 0;
}

/*
 * Clean up the emitter object after a document is dumped.
 */

pub(crate) unsafe fn yaml_emitter_delete_document_and_anchors(emitter: *mut yaml_emitter_t) {
    let mut index: c_int;

    if (*emitter).anchors.is_null() {
        crate::api_document::yaml_document_delete((*emitter).document);
        (*emitter).document = core::ptr::null_mut();
        return;
    }

    index = 0;
    while (*(*emitter).document).nodes.start.offset(index as isize)
        < (*(*emitter).document).nodes.top
    {
        let mut node: yaml_node_t = *(*(*emitter).document).nodes.start.offset(index as isize);
        if (*(*emitter).anchors.offset(index as isize)).serialized == 0 {
            crate::memory::yaml_free(node.tag as *mut c_void);
            if node.type_ == YAML_SCALAR_NODE {
                crate::memory::yaml_free(node.data.scalar.value as *mut c_void);
            }
        }
        if node.type_ == YAML_SEQUENCE_NODE {
            crate::STACK_DEL!(emitter, node.data.sequence.items);
        }
        if node.type_ == YAML_MAPPING_NODE {
            crate::STACK_DEL!(emitter, node.data.mapping.pairs);
        }
        index += 1;
    }

    crate::STACK_DEL!(emitter, (*(*emitter).document).nodes);
    crate::memory::yaml_free((*emitter).anchors as *mut c_void);

    (*emitter).anchors = core::ptr::null_mut();
    (*emitter).last_anchor_id = 0;
    (*emitter).document = core::ptr::null_mut();
}

/*
 * Check the references of a node and assign the anchor id if needed.
 */

pub(crate) unsafe fn yaml_emitter_anchor_node(emitter: *mut yaml_emitter_t, index: c_int) {
    let node: *mut yaml_node_t = (*(*emitter).document)
        .nodes
        .start
        .offset(index as isize - 1);
    let mut item: *mut yaml_node_item_t;
    let mut pair: *mut yaml_node_pair_t;

    (*(*emitter).anchors.offset(index as isize - 1)).references += 1;

    if (*(*emitter).anchors.offset(index as isize - 1)).references == 1 {
        match (*node).type_ {
            YAML_SEQUENCE_NODE => {
                item = (*node).data.sequence.items.start;
                while item < (*node).data.sequence.items.top {
                    yaml_emitter_anchor_node(emitter, *item);
                    item = item.offset(1);
                }
            }
            YAML_MAPPING_NODE => {
                pair = (*node).data.mapping.pairs.start;
                while pair < (*node).data.mapping.pairs.top {
                    yaml_emitter_anchor_node(emitter, (*pair).key);
                    yaml_emitter_anchor_node(emitter, (*pair).value);
                    pair = pair.offset(1);
                }
            }
            _ => {}
        }
    } else if (*(*emitter).anchors.offset(index as isize - 1)).references == 2 {
        (*emitter).last_anchor_id += 1;
        (*(*emitter).anchors.offset(index as isize - 1)).anchor = (*emitter).last_anchor_id;
    }
}

/*
 * Generate a textual representation for an anchor.
 */

/* #define ANCHOR_TEMPLATE         "id%03d" */
/* #define ANCHOR_TEMPLATE_LENGTH  16 */
const ANCHOR_TEMPLATE_LENGTH: size_t = 16;

#[allow(unused_variables)]
pub(crate) unsafe fn yaml_emitter_generate_anchor(
    emitter: *mut yaml_emitter_t,
    anchor_id: c_int,
) -> *mut yaml_char_t {
    let anchor: *mut yaml_char_t =
        crate::memory::yaml_malloc(ANCHOR_TEMPLATE_LENGTH) as *mut yaml_char_t;

    if anchor.is_null() {
        return core::ptr::null_mut();
    }

    /* sprintf((char *)anchor, ANCHOR_TEMPLATE, anchor_id);
     *
     * There is no `sprintf` here, so the "id%03d" conversion is open-coded.
     * `%03d` means: the decimal digits of the value, prefixed with '-' when
     * negative, left-padded with '0' up to a total field width of 3 (the sign
     * counts towards the width).  The widest possible result is
     * "id" + "-" + 10 digits + NUL = 14 bytes, which fits into the 16 bytes
     * that C allocates.
     */
    let negative: bool = anchor_id < 0;
    let mut value: u32 = anchor_id.unsigned_abs();

    /* The decimal digits, least significant first. */
    let mut digits: [u8; 16] = [0; 16];
    let mut ndigits: usize = 0;
    if value == 0 {
        digits[0] = b'0';
        ndigits = 1;
    } else {
        while value != 0 {
            digits[ndigits] = b'0' + (value % 10) as u8;
            value /= 10;
            ndigits += 1;
        }
    }

    let width: usize = ndigits + if negative { 1 } else { 0 };
    let mut padding: usize = if width < 3 { 3 - width } else { 0 };

    let mut pointer: *mut yaml_char_t = anchor;
    *pointer = b'i';
    pointer = pointer.offset(1);
    *pointer = b'd';
    pointer = pointer.offset(1);
    if negative {
        *pointer = b'-';
        pointer = pointer.offset(1);
    }
    while padding != 0 {
        *pointer = b'0';
        pointer = pointer.offset(1);
        padding -= 1;
    }
    while ndigits != 0 {
        ndigits -= 1;
        *pointer = digits[ndigits];
        pointer = pointer.offset(1);
    }
    *pointer = b'\0';

    return anchor;
}

/*
 * Serialize a node.
 */

pub(crate) unsafe fn yaml_emitter_dump_node(emitter: *mut yaml_emitter_t, index: c_int) -> c_int {
    let node: *mut yaml_node_t = (*(*emitter).document)
        .nodes
        .start
        .offset(index as isize - 1);
    let anchor_id: c_int = (*(*emitter).anchors.offset(index as isize - 1)).anchor;
    let mut anchor: *mut yaml_char_t = core::ptr::null_mut();

    if anchor_id != 0 {
        anchor = yaml_emitter_generate_anchor(emitter, anchor_id);
        if anchor.is_null() {
            return 0;
        }
    }

    if (*(*emitter).anchors.offset(index as isize - 1)).serialized != 0 {
        return yaml_emitter_dump_alias(emitter, anchor);
    }

    (*(*emitter).anchors.offset(index as isize - 1)).serialized = 1;

    match (*node).type_ {
        YAML_SCALAR_NODE => {
            return yaml_emitter_dump_scalar(emitter, node, anchor);
        }
        YAML_SEQUENCE_NODE => {
            return yaml_emitter_dump_sequence(emitter, node, anchor);
        }
        YAML_MAPPING_NODE => {
            return yaml_emitter_dump_mapping(emitter, node, anchor);
        }
        _ => {
            yaml_assert!(false); /* Could not happen. */
        }
    }

    return 0; /* Could not happen. */
}

/*
 * Serialize an alias.
 */

pub(crate) unsafe fn yaml_emitter_dump_alias(
    emitter: *mut yaml_emitter_t,
    anchor: *mut yaml_char_t,
) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();
    let mark: yaml_mark_t = yaml_mark_t::zeroed();

    crate::ALIAS_EVENT_INIT!(event, anchor, mark, mark);

    return crate::emitter::yaml_emitter_emit(emitter, &mut event);
}

/*
 * Serialize a scalar.
 */

pub(crate) unsafe fn yaml_emitter_dump_scalar(
    emitter: *mut yaml_emitter_t,
    node: *mut yaml_node_t,
    anchor: *mut yaml_char_t,
) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();
    let mark: yaml_mark_t = yaml_mark_t::zeroed();

    let plain_implicit: c_int = (strcmp(
        (*node).tag as *const c_char,
        YAML_DEFAULT_SCALAR_TAG.as_ptr() as *const c_char,
    ) == 0) as c_int;
    let quoted_implicit: c_int = (strcmp(
        (*node).tag as *const c_char,
        YAML_DEFAULT_SCALAR_TAG.as_ptr() as *const c_char,
    ) == 0) as c_int;

    crate::SCALAR_EVENT_INIT!(
        event,
        anchor,
        (*node).tag,
        (*node).data.scalar.value,
        (*node).data.scalar.length,
        plain_implicit,
        quoted_implicit,
        (*node).data.scalar.style,
        mark,
        mark
    );

    return crate::emitter::yaml_emitter_emit(emitter, &mut event);
}

/*
 * Serialize a sequence.
 */

pub(crate) unsafe fn yaml_emitter_dump_sequence(
    emitter: *mut yaml_emitter_t,
    node: *mut yaml_node_t,
    anchor: *mut yaml_char_t,
) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();
    let mark: yaml_mark_t = yaml_mark_t::zeroed();

    let implicit: c_int = (strcmp(
        (*node).tag as *const c_char,
        YAML_DEFAULT_SEQUENCE_TAG.as_ptr() as *const c_char,
    ) == 0) as c_int;

    let mut item: *mut yaml_node_item_t;

    crate::SEQUENCE_START_EVENT_INIT!(
        event,
        anchor,
        (*node).tag,
        implicit,
        (*node).data.sequence.style,
        mark,
        mark
    );
    if crate::emitter::yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    item = (*node).data.sequence.items.start;
    while item < (*node).data.sequence.items.top {
        if yaml_emitter_dump_node(emitter, *item) == 0 {
            return 0;
        }
        item = item.offset(1);
    }

    crate::SEQUENCE_END_EVENT_INIT!(event, mark, mark);
    if crate::emitter::yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    return 1;
}

/*
 * Serialize a mapping.
 */

pub(crate) unsafe fn yaml_emitter_dump_mapping(
    emitter: *mut yaml_emitter_t,
    node: *mut yaml_node_t,
    anchor: *mut yaml_char_t,
) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();
    let mark: yaml_mark_t = yaml_mark_t::zeroed();

    let implicit: c_int = (strcmp(
        (*node).tag as *const c_char,
        YAML_DEFAULT_MAPPING_TAG.as_ptr() as *const c_char,
    ) == 0) as c_int;

    let mut pair: *mut yaml_node_pair_t;

    crate::MAPPING_START_EVENT_INIT!(
        event,
        anchor,
        (*node).tag,
        implicit,
        (*node).data.mapping.style,
        mark,
        mark
    );
    if crate::emitter::yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    pair = (*node).data.mapping.pairs.start;
    while pair < (*node).data.mapping.pairs.top {
        if yaml_emitter_dump_node(emitter, (*pair).key) == 0 {
            return 0;
        }
        if yaml_emitter_dump_node(emitter, (*pair).value) == 0 {
            return 0;
        }
        pair = pair.offset(1);
    }

    crate::MAPPING_END_EVENT_INIT!(event, mark, mark);
    if crate::emitter::yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    return 1;
}
