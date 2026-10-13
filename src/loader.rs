//! Translation of `c_src/src/loader.c`.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * API functions.
 *
 * YAML_DECLARE(int)
 * yaml_parser_load(yaml_parser_t *parser, yaml_document_t *document);
 */

/*
 * Error handling.
 *
 * static int
 * yaml_parser_set_composer_error(yaml_parser_t *parser,
 *         const char *problem, yaml_mark_t problem_mark);
 *
 * static int
 * yaml_parser_set_composer_error_context(yaml_parser_t *parser,
 *         const char *context, yaml_mark_t context_mark,
 *         const char *problem, yaml_mark_t problem_mark);
 */

/*
 * Alias handling.
 *
 * static int
 * yaml_parser_register_anchor(yaml_parser_t *parser,
 *         int index, yaml_char_t *anchor);
 */

/*
 * Clean up functions.
 *
 * static void
 * yaml_parser_delete_aliases(yaml_parser_t *parser);
 */

/*
 * Document loading context.
 *
 *     struct loader_ctx {
 *         int *start;
 *         int *end;
 *         int *top;
 *     };
 *
 * This has exactly the layout of `yaml_stack_t<int>` (start/end/top), so it is
 * declared as a type alias here: that way the generic `STACK_INIT!`, `PUSH!`,
 * `POP!`, `STACK_EMPTY!`, `STACK_DEL!` and `STACK_LIMIT!` macros apply to it
 * unchanged, exactly as the C macros do.
 */
pub(crate) type loader_ctx = yaml_stack_t<c_int>;

/*
 * Composer functions.
 *
 * static int
 * yaml_parser_load_nodes(yaml_parser_t *parser, struct loader_ctx *ctx);
 *
 * static int
 * yaml_parser_load_document(yaml_parser_t *parser, yaml_event_t *event);
 *
 * static int
 * yaml_parser_load_alias(yaml_parser_t *parser, yaml_event_t *event,
 *         struct loader_ctx *ctx);
 *
 * static int
 * yaml_parser_load_scalar(yaml_parser_t *parser, yaml_event_t *event,
 *         struct loader_ctx *ctx);
 *
 * static int
 * yaml_parser_load_sequence(yaml_parser_t *parser, yaml_event_t *event,
 *         struct loader_ctx *ctx);
 *
 * static int
 * yaml_parser_load_mapping(yaml_parser_t *parser, yaml_event_t *event,
 *         struct loader_ctx *ctx);
 *
 * static int
 * yaml_parser_load_sequence_end(yaml_parser_t *parser, yaml_event_t *event,
 *         struct loader_ctx *ctx);
 *
 * static int
 * yaml_parser_load_mapping_end(yaml_parser_t *parser, yaml_event_t *event,
 *         struct loader_ctx *ctx);
 */

/*
 * Load the next document of the stream.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_load(
    parser: *mut yaml_parser_t,
    document: *mut yaml_document_t,
) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();

    yaml_assert!(!parser.is_null()); /* Non-NULL parser object is expected. */
    yaml_assert!(!document.is_null()); /* Non-NULL document object is expected. */

    memset(
        document as *mut c_void,
        0,
        core::mem::size_of::<yaml_document_t>(),
    );

    'error: {
        if !crate::STACK_INIT!(parser, (*document).nodes) {
            break 'error;
        }

        if (*parser).stream_start_produced == 0 {
            if crate::parser::yaml_parser_parse(parser, &raw mut event) == 0 {
                break 'error;
            }
            yaml_assert!(event.type_ == YAML_STREAM_START_EVENT);
            /* STREAM-START is expected. */
        }

        if (*parser).stream_end_produced != 0 {
            return 1;
        }

        if crate::parser::yaml_parser_parse(parser, &raw mut event) == 0 {
            break 'error;
        }
        if event.type_ == YAML_STREAM_END_EVENT {
            return 1;
        }

        if !crate::STACK_INIT!(parser, (*parser).aliases) {
            break 'error;
        }

        (*parser).document = document;

        if yaml_parser_load_document(parser, &raw mut event) == 0 {
            break 'error;
        }

        yaml_parser_delete_aliases(parser);
        (*parser).document = core::ptr::null_mut();

        return 1;
    }

    /* error: */

    yaml_parser_delete_aliases(parser);
    crate::api_document::yaml_document_delete(document);
    (*parser).document = core::ptr::null_mut();

    0
}

/*
 * Set composer error.
 */

pub(crate) unsafe fn yaml_parser_set_composer_error(
    parser: *mut yaml_parser_t,
    problem: *const c_char,
    problem_mark: yaml_mark_t,
) -> c_int {
    (*parser).error = YAML_COMPOSER_ERROR;
    (*parser).problem = problem;
    (*parser).problem_mark = problem_mark;

    0
}

/*
 * Set composer error with context.
 */

pub(crate) unsafe fn yaml_parser_set_composer_error_context(
    parser: *mut yaml_parser_t,
    context: *const c_char,
    context_mark: yaml_mark_t,
    problem: *const c_char,
    problem_mark: yaml_mark_t,
) -> c_int {
    (*parser).error = YAML_COMPOSER_ERROR;
    (*parser).context = context;
    (*parser).context_mark = context_mark;
    (*parser).problem = problem;
    (*parser).problem_mark = problem_mark;

    0
}

/*
 * Delete the stack of aliases.
 */

pub(crate) unsafe fn yaml_parser_delete_aliases(parser: *mut yaml_parser_t) {
    while !crate::STACK_EMPTY!(parser, (*parser).aliases) {
        crate::memory::yaml_free(crate::POP!(parser, (*parser).aliases).anchor as *mut c_void);
    }
    crate::STACK_DEL!(parser, (*parser).aliases);
}

/*
 * Compose a document object.
 */

pub(crate) unsafe fn yaml_parser_load_document(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let mut ctx: loader_ctx = loader_ctx::zeroed(); /* { NULL, NULL, NULL } */

    yaml_assert!((*event).type_ == YAML_DOCUMENT_START_EVENT);
    /* DOCUMENT-START is expected. */

    (*(*parser).document).version_directive = (*event).data.document_start.version_directive;
    (*(*parser).document).tag_directives.start = (*event).data.document_start.tag_directives.start;
    (*(*parser).document).tag_directives.end = (*event).data.document_start.tag_directives.end;
    (*(*parser).document).start_implicit = (*event).data.document_start.implicit;
    (*(*parser).document).start_mark = (*event).start_mark;

    if !crate::STACK_INIT!(parser, ctx) {
        return 0;
    }
    if yaml_parser_load_nodes(parser, &raw mut ctx) == 0 {
        crate::STACK_DEL!(parser, ctx);
        return 0;
    }
    crate::STACK_DEL!(parser, ctx);

    1
}

/*
 * Compose a node tree.
 */

pub(crate) unsafe fn yaml_parser_load_nodes(
    parser: *mut yaml_parser_t,
    ctx: *mut loader_ctx,
) -> c_int {
    let mut event: yaml_event_t = core::mem::zeroed();

    loop {
        if crate::parser::yaml_parser_parse(parser, &raw mut event) == 0 {
            return 0;
        }

        match event.type_ {
            YAML_ALIAS_EVENT => {
                if yaml_parser_load_alias(parser, &raw mut event, ctx) == 0 {
                    return 0;
                }
            }
            YAML_SCALAR_EVENT => {
                if yaml_parser_load_scalar(parser, &raw mut event, ctx) == 0 {
                    return 0;
                }
            }
            YAML_SEQUENCE_START_EVENT => {
                if yaml_parser_load_sequence(parser, &raw mut event, ctx) == 0 {
                    return 0;
                }
            }
            YAML_SEQUENCE_END_EVENT => {
                if yaml_parser_load_sequence_end(parser, &raw mut event, ctx) == 0 {
                    return 0;
                }
            }
            YAML_MAPPING_START_EVENT => {
                if yaml_parser_load_mapping(parser, &raw mut event, ctx) == 0 {
                    return 0;
                }
            }
            YAML_MAPPING_END_EVENT => {
                if yaml_parser_load_mapping_end(parser, &raw mut event, ctx) == 0 {
                    return 0;
                }
            }
            YAML_DOCUMENT_END_EVENT => {}
            _ => {
                yaml_assert!(false); /* Could not happen. */
                return 0;
            }
        }

        if !(event.type_ != YAML_DOCUMENT_END_EVENT) {
            break;
        }
    }

    (*(*parser).document).end_implicit = event.data.document_end.implicit;
    (*(*parser).document).end_mark = event.end_mark;

    1
}

/*
 * Add an anchor.
 */

pub(crate) unsafe fn yaml_parser_register_anchor(
    parser: *mut yaml_parser_t,
    index: c_int,
    anchor: *mut yaml_char_t,
) -> c_int {
    let mut data: yaml_alias_data_t = core::mem::zeroed();
    let mut alias_data: *mut yaml_alias_data_t;

    if anchor.is_null() {
        return 1;
    }

    data.anchor = anchor;
    data.index = index;
    data.mark = (*(*(*parser).document)
        .nodes
        .start
        .offset((index - 1) as isize))
    .start_mark;

    alias_data = (*parser).aliases.start;
    while alias_data != (*parser).aliases.top {
        if strcmp(
            (*alias_data).anchor as *const c_char,
            anchor as *const c_char,
        ) == 0
        {
            crate::memory::yaml_free(anchor as *mut c_void);
            return yaml_parser_set_composer_error_context(
                parser,
                b"found duplicate anchor; first occurrence\0".as_ptr() as *const c_char,
                (*alias_data).mark,
                b"second occurrence\0".as_ptr() as *const c_char,
                data.mark,
            );
        }
        alias_data = alias_data.add(1);
    }

    if !crate::PUSH!(parser, (*parser).aliases, data) {
        crate::memory::yaml_free(anchor as *mut c_void);
        return 0;
    }

    1
}

/*
 * Compose node into its parent in the stree.
 */

pub(crate) unsafe fn yaml_parser_load_node_add(
    parser: *mut yaml_parser_t,
    ctx: *mut loader_ctx,
    index: c_int,
) -> c_int {
    let parent: *mut yaml_node_t;
    let parent_index: c_int;

    if crate::STACK_EMPTY!(parser, (*ctx)) {
        /* This is the root node, there's no tree to add it to. */
        return 1;
    }

    parent_index = *(*ctx).top.offset(-1);
    parent = (*(*parser).document)
        .nodes
        .start
        .offset((parent_index - 1) as isize);

    match (*parent).type_ {
        YAML_SEQUENCE_NODE => {
            if !crate::STACK_LIMIT!(parser, (*parent).data.sequence.items, INT_MAX - 1) {
                return 0;
            }
            if !crate::PUSH!(parser, (*parent).data.sequence.items, index) {
                return 0;
            }
        }
        YAML_MAPPING_NODE => 'mapping: {
            let mut pair: yaml_node_pair_t = core::mem::zeroed();
            if !crate::STACK_EMPTY!(parser, (*parent).data.mapping.pairs) {
                let p: *mut yaml_node_pair_t = (*parent).data.mapping.pairs.top.offset(-1);
                if (*p).key != 0 && (*p).value == 0 {
                    (*p).value = index;
                    break 'mapping;
                }
            }

            pair.key = index;
            pair.value = 0;
            if !crate::STACK_LIMIT!(parser, (*parent).data.mapping.pairs, INT_MAX - 1) {
                return 0;
            }
            if !crate::PUSH!(parser, (*parent).data.mapping.pairs, pair) {
                return 0;
            }
        }
        _ => {
            yaml_assert!(false); /* Could not happen. */
            return 0;
        }
    }
    1
}

/*
 * Compose a node corresponding to an alias.
 */

pub(crate) unsafe fn yaml_parser_load_alias(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut loader_ctx,
) -> c_int {
    let anchor: *mut yaml_char_t = (*event).data.alias.anchor;
    let mut alias_data: *mut yaml_alias_data_t;

    alias_data = (*parser).aliases.start;
    while alias_data != (*parser).aliases.top {
        if strcmp(
            (*alias_data).anchor as *const c_char,
            anchor as *const c_char,
        ) == 0
        {
            crate::memory::yaml_free(anchor as *mut c_void);
            return yaml_parser_load_node_add(parser, ctx, (*alias_data).index);
        }
        alias_data = alias_data.add(1);
    }

    crate::memory::yaml_free(anchor as *mut c_void);
    yaml_parser_set_composer_error(
        parser,
        b"found undefined alias\0".as_ptr() as *const c_char,
        (*event).start_mark,
    )
}

/*
 * Compose a scalar node.
 */

pub(crate) unsafe fn yaml_parser_load_scalar(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut loader_ctx,
) -> c_int {
    let mut node: yaml_node_t = core::mem::zeroed();
    let index: c_int;
    let mut tag: *mut yaml_char_t = (*event).data.scalar.tag;

    'error: {
        if !crate::STACK_LIMIT!(parser, (*(*parser).document).nodes, INT_MAX - 1) {
            break 'error;
        }

        if tag.is_null() || strcmp(tag as *const c_char, b"!\0".as_ptr() as *const c_char) == 0 {
            crate::memory::yaml_free(tag as *mut c_void);
            tag = crate::memory::yaml_strdup(
                YAML_DEFAULT_SCALAR_TAG.as_ptr() as *const yaml_char_t
            );
            if tag.is_null() {
                break 'error;
            }
        }

        crate::SCALAR_NODE_INIT!(
            node,
            tag,
            (*event).data.scalar.value,
            (*event).data.scalar.length,
            (*event).data.scalar.style,
            (*event).start_mark,
            (*event).end_mark
        );

        if !crate::PUSH!(parser, (*(*parser).document).nodes, node) {
            break 'error;
        }

        index = (*(*parser).document)
            .nodes
            .top
            .offset_from((*(*parser).document).nodes.start) as c_int;

        if yaml_parser_register_anchor(parser, index, (*event).data.scalar.anchor) == 0 {
            return 0;
        }

        return yaml_parser_load_node_add(parser, ctx, index);
    }

    /* error: */
    crate::memory::yaml_free(tag as *mut c_void);
    crate::memory::yaml_free((*event).data.scalar.anchor as *mut c_void);
    crate::memory::yaml_free((*event).data.scalar.value as *mut c_void);
    0
}

/*
 * Compose a sequence node.
 */

pub(crate) unsafe fn yaml_parser_load_sequence(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut loader_ctx,
) -> c_int {
    let mut node: yaml_node_t = core::mem::zeroed();
    let mut items: yaml_stack_t<yaml_node_item_t> = yaml_stack_t::zeroed(); /* { NULL, NULL, NULL } */
    let index: c_int;
    let mut tag: *mut yaml_char_t = (*event).data.sequence_start.tag;

    'error: {
        if !crate::STACK_LIMIT!(parser, (*(*parser).document).nodes, INT_MAX - 1) {
            break 'error;
        }

        if tag.is_null() || strcmp(tag as *const c_char, b"!\0".as_ptr() as *const c_char) == 0 {
            crate::memory::yaml_free(tag as *mut c_void);
            tag = crate::memory::yaml_strdup(
                YAML_DEFAULT_SEQUENCE_TAG.as_ptr() as *const yaml_char_t
            );
            if tag.is_null() {
                break 'error;
            }
        }

        if !crate::STACK_INIT!(parser, items) {
            break 'error;
        }

        crate::SEQUENCE_NODE_INIT!(
            node,
            tag,
            items.start,
            items.end,
            (*event).data.sequence_start.style,
            (*event).start_mark,
            (*event).end_mark
        );

        if !crate::PUSH!(parser, (*(*parser).document).nodes, node) {
            break 'error;
        }

        index = (*(*parser).document)
            .nodes
            .top
            .offset_from((*(*parser).document).nodes.start) as c_int;

        if yaml_parser_register_anchor(parser, index, (*event).data.sequence_start.anchor) == 0 {
            return 0;
        }

        if yaml_parser_load_node_add(parser, ctx, index) == 0 {
            return 0;
        }

        if !crate::STACK_LIMIT!(parser, (*ctx), INT_MAX - 1) {
            return 0;
        }
        if !crate::PUSH!(parser, (*ctx), index) {
            return 0;
        }

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(tag as *mut c_void);
    crate::memory::yaml_free((*event).data.sequence_start.anchor as *mut c_void);
    0
}

pub(crate) unsafe fn yaml_parser_load_sequence_end(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut loader_ctx,
) -> c_int {
    let index: c_int;

    yaml_assert!(((*ctx).top.offset_from((*ctx).start)) > 0);

    index = *(*ctx).top.offset(-1);
    yaml_assert!(
        (*(*(*parser).document)
            .nodes
            .start
            .offset((index - 1) as isize))
        .type_
            == YAML_SEQUENCE_NODE
    );
    (*(*(*parser).document)
        .nodes
        .start
        .offset((index - 1) as isize))
    .end_mark = (*event).end_mark;

    let _ = crate::POP!(parser, (*ctx));

    1
}

/*
 * Compose a mapping node.
 */

pub(crate) unsafe fn yaml_parser_load_mapping(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut loader_ctx,
) -> c_int {
    let mut node: yaml_node_t = core::mem::zeroed();
    let mut pairs: yaml_stack_t<yaml_node_pair_t> = yaml_stack_t::zeroed(); /* { NULL, NULL, NULL } */
    let index: c_int;
    let mut tag: *mut yaml_char_t = (*event).data.mapping_start.tag;

    'error: {
        if !crate::STACK_LIMIT!(parser, (*(*parser).document).nodes, INT_MAX - 1) {
            break 'error;
        }

        if tag.is_null() || strcmp(tag as *const c_char, b"!\0".as_ptr() as *const c_char) == 0 {
            crate::memory::yaml_free(tag as *mut c_void);
            tag = crate::memory::yaml_strdup(
                YAML_DEFAULT_MAPPING_TAG.as_ptr() as *const yaml_char_t
            );
            if tag.is_null() {
                break 'error;
            }
        }

        if !crate::STACK_INIT!(parser, pairs) {
            break 'error;
        }

        crate::MAPPING_NODE_INIT!(
            node,
            tag,
            pairs.start,
            pairs.end,
            (*event).data.mapping_start.style,
            (*event).start_mark,
            (*event).end_mark
        );

        if !crate::PUSH!(parser, (*(*parser).document).nodes, node) {
            break 'error;
        }

        index = (*(*parser).document)
            .nodes
            .top
            .offset_from((*(*parser).document).nodes.start) as c_int;

        if yaml_parser_register_anchor(parser, index, (*event).data.mapping_start.anchor) == 0 {
            return 0;
        }

        if yaml_parser_load_node_add(parser, ctx, index) == 0 {
            return 0;
        }

        if !crate::STACK_LIMIT!(parser, (*ctx), INT_MAX - 1) {
            return 0;
        }
        if !crate::PUSH!(parser, (*ctx), index) {
            return 0;
        }

        return 1;
    }

    /* error: */
    crate::memory::yaml_free(tag as *mut c_void);
    crate::memory::yaml_free((*event).data.mapping_start.anchor as *mut c_void);
    0
}

pub(crate) unsafe fn yaml_parser_load_mapping_end(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut loader_ctx,
) -> c_int {
    let index: c_int;

    yaml_assert!(((*ctx).top.offset_from((*ctx).start)) > 0);

    index = *(*ctx).top.offset(-1);
    yaml_assert!(
        (*(*(*parser).document)
            .nodes
            .start
            .offset((index - 1) as isize))
        .type_
            == YAML_MAPPING_NODE
    );
    (*(*(*parser).document)
        .nodes
        .start
        .offset((index - 1) as isize))
    .end_mark = (*event).end_mark;

    let _ = crate::POP!(parser, (*ctx));

    1
}
