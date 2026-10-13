//! Translation of `c_src/src/api.c` lines 1034-1393: document construction
//! and node manipulation.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/// The anonymous local `struct { yaml_error_type_t error; } context;` that the
/// functions below declare so they can use the stack macros (which report
/// allocation failures through `context->error`).
#[repr(C)]
pub(crate) struct ContextErr {
    pub error: yaml_error_type_t,
}

/*
 * Create a document object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_initialize(
    document: *mut yaml_document_t,
    version_directive: *mut yaml_version_directive_t,
    tag_directives_start: *mut yaml_tag_directive_t,
    tag_directives_end: *mut yaml_tag_directive_t,
    start_implicit: c_int,
    end_implicit: c_int,
) -> c_int {
    let mut context = ContextErr { error: YAML_NO_ERROR };
    let context = &raw mut context;
    let mut nodes: yaml_stack_t<yaml_node_t> = yaml_stack_t::zeroed();
    let mut version_directive_copy: *mut yaml_version_directive_t = core::ptr::null_mut();
    let mut tag_directives_copy: yaml_stack_t<yaml_tag_directive_t> = yaml_stack_t::zeroed();
    let mut value: yaml_tag_directive_t = yaml_tag_directive_t {
        handle: core::ptr::null_mut(),
        prefix: core::ptr::null_mut(),
    };
    let mark: yaml_mark_t = yaml_mark_t::zeroed();

    yaml_assert!(!document.is_null()); /* Non-NULL document object is expected. */
    yaml_assert!(
        (!tag_directives_start.is_null() && !tag_directives_end.is_null())
            || (tag_directives_start == tag_directives_end)
    );
    /* Valid tag directives are expected. */

    'error: {
        if !crate::STACK_INIT!(context, nodes) {
            break 'error;
        }

        if !version_directive.is_null() {
            version_directive_copy = crate::memory::yaml_malloc(core::mem::size_of::<
                yaml_version_directive_t,
            >()) as *mut yaml_version_directive_t;
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
                if crate::api_objects::yaml_check_utf8(
                    (*tag_directive).handle,
                    strlen((*tag_directive).handle as *const c_char),
                ) == 0
                {
                    break 'error;
                }
                if crate::api_objects::yaml_check_utf8(
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

        crate::DOCUMENT_INIT!(
            (*document),
            nodes.start,
            nodes.end,
            version_directive_copy,
            tag_directives_copy.start,
            tag_directives_copy.top,
            start_implicit,
            end_implicit,
            mark,
            mark
        );

        return 1;
    }

    /* error: */
    crate::STACK_DEL!(context, nodes);
    crate::memory::yaml_free(version_directive_copy as *mut c_void);
    while !crate::STACK_EMPTY!(context, tag_directives_copy) {
        let value: yaml_tag_directive_t = crate::POP!(context, tag_directives_copy);
        crate::memory::yaml_free(value.handle as *mut c_void);
        crate::memory::yaml_free(value.prefix as *mut c_void);
    }
    crate::STACK_DEL!(context, tag_directives_copy);
    crate::memory::yaml_free(value.handle as *mut c_void);
    crate::memory::yaml_free(value.prefix as *mut c_void);

    0
}

/*
 * Destroy a document object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_delete(document: *mut yaml_document_t) {
    let mut tag_directive: *mut yaml_tag_directive_t;

    /* The C original passes `&context` to STACK_EMPTY/POP/STACK_DEL without
     * declaring one: those macros discard the argument.  We keep a dummy so the
     * macro call sites look the same. */
    let mut context = ContextErr { error: YAML_NO_ERROR };
    let context = &raw mut context;
    let _ = context;

    yaml_assert!(!document.is_null()); /* Non-NULL document object is expected. */

    while !crate::STACK_EMPTY!(context, (*document).nodes) {
        let mut node: yaml_node_t = crate::POP!(context, (*document).nodes);
        crate::memory::yaml_free(node.tag as *mut c_void);
        match node.type_ {
            YAML_SCALAR_NODE => {
                crate::memory::yaml_free(node.data.scalar.value as *mut c_void);
            }
            YAML_SEQUENCE_NODE => {
                crate::STACK_DEL!(context, node.data.sequence.items);
            }
            YAML_MAPPING_NODE => {
                crate::STACK_DEL!(context, node.data.mapping.pairs);
            }
            _ => {
                yaml_assert!(false); /* Should not happen. */
            }
        }
    }
    crate::STACK_DEL!(context, (*document).nodes);

    crate::memory::yaml_free((*document).version_directive as *mut c_void);
    tag_directive = (*document).tag_directives.start;
    while tag_directive != (*document).tag_directives.end {
        crate::memory::yaml_free((*tag_directive).handle as *mut c_void);
        crate::memory::yaml_free((*tag_directive).prefix as *mut c_void);
        tag_directive = tag_directive.add(1);
    }
    crate::memory::yaml_free((*document).tag_directives.start as *mut c_void);

    memset(
        document as *mut c_void,
        0,
        core::mem::size_of::<yaml_document_t>(),
    );
}

/**
 * Get a document node.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_get_node(
    document: *mut yaml_document_t,
    index: c_int,
) -> *mut yaml_node_t {
    yaml_assert!(!document.is_null()); /* Non-NULL document object is expected. */

    if index > 0 && (*document).nodes.start.offset(index as isize) <= (*document).nodes.top {
        return (*document).nodes.start.offset(index as isize - 1);
    }
    core::ptr::null_mut()
}

/**
 * Get the root object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_get_root_node(
    document: *mut yaml_document_t,
) -> *mut yaml_node_t {
    yaml_assert!(!document.is_null()); /* Non-NULL document object is expected. */

    if (*document).nodes.top != (*document).nodes.start {
        return (*document).nodes.start;
    }
    core::ptr::null_mut()
}

/*
 * Add a scalar node to a document.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_add_scalar(
    document: *mut yaml_document_t,
    mut tag: *const yaml_char_t,
    value: *const yaml_char_t,
    mut length: c_int,
    style: yaml_scalar_style_t,
) -> c_int {
    let mut context = ContextErr { error: YAML_NO_ERROR };
    let context = &raw mut context;
    let mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut tag_copy: *mut yaml_char_t = core::ptr::null_mut();
    let mut value_copy: *mut yaml_char_t = core::ptr::null_mut();
    let mut node: yaml_node_t = core::mem::zeroed();

    yaml_assert!(!document.is_null()); /* Non-NULL document object is expected. */
    yaml_assert!(!value.is_null()); /* Non-NULL value is expected. */

    'error: {
        if tag.is_null() {
            tag = YAML_DEFAULT_SCALAR_TAG.as_ptr() as *mut yaml_char_t;
        }

        if crate::api_objects::yaml_check_utf8(tag, strlen(tag as *const c_char)) == 0 {
            break 'error;
        }
        tag_copy = crate::memory::yaml_strdup(tag);
        if tag_copy.is_null() {
            break 'error;
        }

        if length < 0 {
            length = strlen(value as *const c_char) as c_int;
        }

        if crate::api_objects::yaml_check_utf8(value, length as size_t) == 0 {
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

        crate::SCALAR_NODE_INIT!(
            node,
            tag_copy,
            value_copy,
            length as size_t,
            style,
            mark,
            mark
        );
        if !crate::PUSH!(context, (*document).nodes, node) {
            break 'error;
        }

        return (*document).nodes.top.offset_from((*document).nodes.start) as c_int;
    }

    /* error: */
    crate::memory::yaml_free(tag_copy as *mut c_void);
    crate::memory::yaml_free(value_copy as *mut c_void);

    0
}

/*
 * Add a sequence node to a document.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_add_sequence(
    document: *mut yaml_document_t,
    mut tag: *const yaml_char_t,
    style: yaml_sequence_style_t,
) -> c_int {
    let mut context = ContextErr { error: YAML_NO_ERROR };
    let context = &raw mut context;
    let mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut tag_copy: *mut yaml_char_t = core::ptr::null_mut();
    let mut items: yaml_stack_t<yaml_node_item_t> = yaml_stack_t::zeroed();
    let mut node: yaml_node_t = core::mem::zeroed();

    yaml_assert!(!document.is_null()); /* Non-NULL document object is expected. */

    'error: {
        if tag.is_null() {
            tag = YAML_DEFAULT_SEQUENCE_TAG.as_ptr() as *mut yaml_char_t;
        }

        if crate::api_objects::yaml_check_utf8(tag, strlen(tag as *const c_char)) == 0 {
            break 'error;
        }
        tag_copy = crate::memory::yaml_strdup(tag);
        if tag_copy.is_null() {
            break 'error;
        }

        if !crate::STACK_INIT!(context, items) {
            break 'error;
        }

        crate::SEQUENCE_NODE_INIT!(node, tag_copy, items.start, items.end, style, mark, mark);
        if !crate::PUSH!(context, (*document).nodes, node) {
            break 'error;
        }

        return (*document).nodes.top.offset_from((*document).nodes.start) as c_int;
    }

    /* error: */
    crate::STACK_DEL!(context, items);
    crate::memory::yaml_free(tag_copy as *mut c_void);

    0
}

/*
 * Add a mapping node to a document.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_add_mapping(
    document: *mut yaml_document_t,
    mut tag: *const yaml_char_t,
    style: yaml_mapping_style_t,
) -> c_int {
    let mut context = ContextErr { error: YAML_NO_ERROR };
    let context = &raw mut context;
    let mark: yaml_mark_t = yaml_mark_t::zeroed();
    let mut tag_copy: *mut yaml_char_t = core::ptr::null_mut();
    let mut pairs: yaml_stack_t<yaml_node_pair_t> = yaml_stack_t::zeroed();
    let mut node: yaml_node_t = core::mem::zeroed();

    yaml_assert!(!document.is_null()); /* Non-NULL document object is expected. */

    'error: {
        if tag.is_null() {
            tag = YAML_DEFAULT_MAPPING_TAG.as_ptr() as *mut yaml_char_t;
        }

        if crate::api_objects::yaml_check_utf8(tag, strlen(tag as *const c_char)) == 0 {
            break 'error;
        }
        tag_copy = crate::memory::yaml_strdup(tag);
        if tag_copy.is_null() {
            break 'error;
        }

        if !crate::STACK_INIT!(context, pairs) {
            break 'error;
        }

        crate::MAPPING_NODE_INIT!(node, tag_copy, pairs.start, pairs.end, style, mark, mark);
        if !crate::PUSH!(context, (*document).nodes, node) {
            break 'error;
        }

        return (*document).nodes.top.offset_from((*document).nodes.start) as c_int;
    }

    /* error: */
    crate::STACK_DEL!(context, pairs);
    crate::memory::yaml_free(tag_copy as *mut c_void);

    0
}

/*
 * Append an item to a sequence node.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_append_sequence_item(
    document: *mut yaml_document_t,
    sequence: c_int,
    item: c_int,
) -> c_int {
    let mut context = ContextErr { error: YAML_NO_ERROR };
    let context = &raw mut context;

    yaml_assert!(!document.is_null()); /* Non-NULL document is required. */
    yaml_assert!(
        sequence > 0 && (*document).nodes.start.offset(sequence as isize) <= (*document).nodes.top
    );
    /* Valid sequence id is required. */
    yaml_assert!(
        (*(*document).nodes.start.offset(sequence as isize - 1)).type_ == YAML_SEQUENCE_NODE
    );
    /* A sequence node is required. */
    yaml_assert!(item > 0 && (*document).nodes.start.offset(item as isize) <= (*document).nodes.top);
    /* Valid item id is required. */

    if !crate::PUSH!(
        context,
        (*(*document).nodes.start.offset(sequence as isize - 1))
            .data
            .sequence
            .items,
        item
    ) {
        return 0;
    }

    1
}

/*
 * Append a pair of a key and a value to a mapping node.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_document_append_mapping_pair(
    document: *mut yaml_document_t,
    mapping: c_int,
    key: c_int,
    value: c_int,
) -> c_int {
    let mut context = ContextErr { error: YAML_NO_ERROR };
    let context = &raw mut context;

    let mut pair: yaml_node_pair_t = core::mem::zeroed();

    yaml_assert!(!document.is_null()); /* Non-NULL document is required. */
    yaml_assert!(
        mapping > 0 && (*document).nodes.start.offset(mapping as isize) <= (*document).nodes.top
    );
    /* Valid mapping id is required. */
    yaml_assert!(
        (*(*document).nodes.start.offset(mapping as isize - 1)).type_ == YAML_MAPPING_NODE
    );
    /* A mapping node is required. */
    yaml_assert!(key > 0 && (*document).nodes.start.offset(key as isize) <= (*document).nodes.top);
    /* Valid key id is required. */
    yaml_assert!(
        value > 0 && (*document).nodes.start.offset(value as isize) <= (*document).nodes.top
    );
    /* Valid value id is required. */

    pair.key = key;
    pair.value = value;

    if !crate::PUSH!(
        context,
        (*(*document).nodes.start.offset(mapping as isize - 1))
            .data
            .mapping
            .pairs,
        pair
    ) {
        return 0;
    }

    1
}
