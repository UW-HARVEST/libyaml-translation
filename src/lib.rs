//! A faithful Rust translation of libyaml 0.2.5.
//!
//! Layout and ABI follow `c_src/include/yaml.h` exactly: every public type is
//! `#[repr(C)]` with the same field order, and every `YAML_DECLARE` function is
//! re-exported under its original linker symbol name.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::all)]

pub mod libc_shim;
pub mod memory;
#[macro_use]
pub mod private;

pub mod api;
pub mod api_document;
pub mod api_objects;
pub mod dumper;
pub mod emitter;
pub mod emitter_analyze;
pub mod emitter_write;
pub mod loader;
pub mod parser;
pub mod parser_states;
pub mod reader;
pub mod scanner;
pub mod scanner_scalar;
pub mod scanner_scan;
pub mod scanner_tokens;
pub mod writer;

use libc_shim::{c_int, c_uchar, c_uint, c_void, size_t, FILE};

/* ------------------------------------------------------------------ */
/* Version information (cmake/config.h.in)                            */
/* ------------------------------------------------------------------ */

pub const YAML_VERSION_MAJOR: c_int = 0;
pub const YAML_VERSION_MINOR: c_int = 2;
pub const YAML_VERSION_PATCH: c_int = 5;
/// NUL-terminated so a `*const c_char` to it can be handed straight to C.
pub const YAML_VERSION_STRING: &[u8] = b"0.2.5\0";

/* ------------------------------------------------------------------ */
/* Basic types                                                         */
/* ------------------------------------------------------------------ */

/// The character type (UTF-8 octet).
pub type yaml_char_t = c_uchar;

/// The version directive data.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_version_directive_t {
    /// The major version number.
    pub major: c_int,
    /// The minor version number.
    pub minor: c_int,
}

/// The tag directive data.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_tag_directive_t {
    /// The tag handle.
    pub handle: *mut yaml_char_t,
    /// The tag prefix.
    pub prefix: *mut yaml_char_t,
}

/// The stream encoding.
pub type yaml_encoding_t = c_uint;
pub const YAML_ANY_ENCODING: yaml_encoding_t = 0;
pub const YAML_UTF8_ENCODING: yaml_encoding_t = 1;
pub const YAML_UTF16LE_ENCODING: yaml_encoding_t = 2;
pub const YAML_UTF16BE_ENCODING: yaml_encoding_t = 3;

/// Line break types.
pub type yaml_break_t = c_uint;
pub const YAML_ANY_BREAK: yaml_break_t = 0;
pub const YAML_CR_BREAK: yaml_break_t = 1;
pub const YAML_LN_BREAK: yaml_break_t = 2;
pub const YAML_CRLN_BREAK: yaml_break_t = 3;

/// Many bad things could happen with the parser and emitter.
pub type yaml_error_type_t = c_uint;
pub const YAML_NO_ERROR: yaml_error_type_t = 0;
pub const YAML_MEMORY_ERROR: yaml_error_type_t = 1;
pub const YAML_READER_ERROR: yaml_error_type_t = 2;
pub const YAML_SCANNER_ERROR: yaml_error_type_t = 3;
pub const YAML_PARSER_ERROR: yaml_error_type_t = 4;
pub const YAML_COMPOSER_ERROR: yaml_error_type_t = 5;
pub const YAML_WRITER_ERROR: yaml_error_type_t = 6;
pub const YAML_EMITTER_ERROR: yaml_error_type_t = 7;

/// The pointer position.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_mark_t {
    /// The position index.
    pub index: size_t,
    /// The position line.
    pub line: size_t,
    /// The position column.
    pub column: size_t,
}

impl yaml_mark_t {
    pub const fn zeroed() -> Self {
        yaml_mark_t {
            index: 0,
            line: 0,
            column: 0,
        }
    }
}

/* ------------------------------------------------------------------ */
/* Node styles                                                         */
/* ------------------------------------------------------------------ */

/// Scalar styles.
pub type yaml_scalar_style_t = c_uint;
pub const YAML_ANY_SCALAR_STYLE: yaml_scalar_style_t = 0;
pub const YAML_PLAIN_SCALAR_STYLE: yaml_scalar_style_t = 1;
pub const YAML_SINGLE_QUOTED_SCALAR_STYLE: yaml_scalar_style_t = 2;
pub const YAML_DOUBLE_QUOTED_SCALAR_STYLE: yaml_scalar_style_t = 3;
pub const YAML_LITERAL_SCALAR_STYLE: yaml_scalar_style_t = 4;
pub const YAML_FOLDED_SCALAR_STYLE: yaml_scalar_style_t = 5;

/// Sequence styles.
pub type yaml_sequence_style_t = c_uint;
pub const YAML_ANY_SEQUENCE_STYLE: yaml_sequence_style_t = 0;
pub const YAML_BLOCK_SEQUENCE_STYLE: yaml_sequence_style_t = 1;
pub const YAML_FLOW_SEQUENCE_STYLE: yaml_sequence_style_t = 2;

/// Mapping styles.
pub type yaml_mapping_style_t = c_uint;
pub const YAML_ANY_MAPPING_STYLE: yaml_mapping_style_t = 0;
pub const YAML_BLOCK_MAPPING_STYLE: yaml_mapping_style_t = 1;
pub const YAML_FLOW_MAPPING_STYLE: yaml_mapping_style_t = 2;

/* ------------------------------------------------------------------ */
/* Generic stack / queue / buffer shapes                               */
/* ------------------------------------------------------------------ */

/// `struct { T *start; T *end; T *top; }`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_stack_t<T> {
    pub start: *mut T,
    pub end: *mut T,
    pub top: *mut T,
}

impl<T> yaml_stack_t<T> {
    pub const fn zeroed() -> Self {
        yaml_stack_t {
            start: core::ptr::null_mut(),
            end: core::ptr::null_mut(),
            top: core::ptr::null_mut(),
        }
    }
}

/// `struct { T *start; T *end; T *head; T *tail; }`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_queue_t<T> {
    pub start: *mut T,
    pub end: *mut T,
    pub head: *mut T,
    pub tail: *mut T,
}

impl<T> yaml_queue_t<T> {
    pub const fn zeroed() -> Self {
        yaml_queue_t {
            start: core::ptr::null_mut(),
            end: core::ptr::null_mut(),
            head: core::ptr::null_mut(),
            tail: core::ptr::null_mut(),
        }
    }
}

/// `struct { yaml_char_t *start; yaml_char_t *end; yaml_char_t *pointer; yaml_char_t *last; }`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_buffer_t {
    pub start: *mut yaml_char_t,
    pub end: *mut yaml_char_t,
    pub pointer: *mut yaml_char_t,
    pub last: *mut yaml_char_t,
}

impl yaml_buffer_t {
    pub const fn zeroed() -> Self {
        yaml_buffer_t {
            start: core::ptr::null_mut(),
            end: core::ptr::null_mut(),
            pointer: core::ptr::null_mut(),
            last: core::ptr::null_mut(),
        }
    }
}

/// `struct { yaml_tag_directive_t *start; yaml_tag_directive_t *end; }`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_tag_directives_range_t {
    pub start: *mut yaml_tag_directive_t,
    pub end: *mut yaml_tag_directive_t,
}

/* ------------------------------------------------------------------ */
/* Tokens                                                              */
/* ------------------------------------------------------------------ */

/// Token types.
pub type yaml_token_type_t = c_uint;
pub const YAML_NO_TOKEN: yaml_token_type_t = 0;
pub const YAML_STREAM_START_TOKEN: yaml_token_type_t = 1;
pub const YAML_STREAM_END_TOKEN: yaml_token_type_t = 2;
pub const YAML_VERSION_DIRECTIVE_TOKEN: yaml_token_type_t = 3;
pub const YAML_TAG_DIRECTIVE_TOKEN: yaml_token_type_t = 4;
pub const YAML_DOCUMENT_START_TOKEN: yaml_token_type_t = 5;
pub const YAML_DOCUMENT_END_TOKEN: yaml_token_type_t = 6;
pub const YAML_BLOCK_SEQUENCE_START_TOKEN: yaml_token_type_t = 7;
pub const YAML_BLOCK_MAPPING_START_TOKEN: yaml_token_type_t = 8;
pub const YAML_BLOCK_END_TOKEN: yaml_token_type_t = 9;
pub const YAML_FLOW_SEQUENCE_START_TOKEN: yaml_token_type_t = 10;
pub const YAML_FLOW_SEQUENCE_END_TOKEN: yaml_token_type_t = 11;
pub const YAML_FLOW_MAPPING_START_TOKEN: yaml_token_type_t = 12;
pub const YAML_FLOW_MAPPING_END_TOKEN: yaml_token_type_t = 13;
pub const YAML_BLOCK_ENTRY_TOKEN: yaml_token_type_t = 14;
pub const YAML_FLOW_ENTRY_TOKEN: yaml_token_type_t = 15;
pub const YAML_KEY_TOKEN: yaml_token_type_t = 16;
pub const YAML_VALUE_TOKEN: yaml_token_type_t = 17;
pub const YAML_ALIAS_TOKEN: yaml_token_type_t = 18;
pub const YAML_ANCHOR_TOKEN: yaml_token_type_t = 19;
pub const YAML_TAG_TOKEN: yaml_token_type_t = 20;
pub const YAML_SCALAR_TOKEN: yaml_token_type_t = 21;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_stream_start_t {
    pub encoding: yaml_encoding_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_alias_t {
    pub value: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_anchor_t {
    pub value: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_tag_t {
    pub handle: *mut yaml_char_t,
    pub suffix: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_scalar_t {
    pub value: *mut yaml_char_t,
    pub length: size_t,
    pub style: yaml_scalar_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_version_directive_t {
    pub major: c_int,
    pub minor: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_tag_directive_t {
    pub handle: *mut yaml_char_t,
    pub prefix: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_token_data_t {
    pub stream_start: yaml_token_data_stream_start_t,
    pub alias: yaml_token_data_alias_t,
    pub anchor: yaml_token_data_anchor_t,
    pub tag: yaml_token_data_tag_t,
    pub scalar: yaml_token_data_scalar_t,
    pub version_directive: yaml_token_data_version_directive_t,
    pub tag_directive: yaml_token_data_tag_directive_t,
}

/// The token structure.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_t {
    /// The token type.
    pub type_: yaml_token_type_t,
    /// The token data.
    pub data: yaml_token_data_t,
    /// The beginning of the token.
    pub start_mark: yaml_mark_t,
    /// The end of the token.
    pub end_mark: yaml_mark_t,
}

/* ------------------------------------------------------------------ */
/* Events                                                              */
/* ------------------------------------------------------------------ */

/// Event types.
pub type yaml_event_type_t = c_uint;
pub const YAML_NO_EVENT: yaml_event_type_t = 0;
pub const YAML_STREAM_START_EVENT: yaml_event_type_t = 1;
pub const YAML_STREAM_END_EVENT: yaml_event_type_t = 2;
pub const YAML_DOCUMENT_START_EVENT: yaml_event_type_t = 3;
pub const YAML_DOCUMENT_END_EVENT: yaml_event_type_t = 4;
pub const YAML_ALIAS_EVENT: yaml_event_type_t = 5;
pub const YAML_SCALAR_EVENT: yaml_event_type_t = 6;
pub const YAML_SEQUENCE_START_EVENT: yaml_event_type_t = 7;
pub const YAML_SEQUENCE_END_EVENT: yaml_event_type_t = 8;
pub const YAML_MAPPING_START_EVENT: yaml_event_type_t = 9;
pub const YAML_MAPPING_END_EVENT: yaml_event_type_t = 10;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_stream_start_t {
    pub encoding: yaml_encoding_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_document_start_t {
    pub version_directive: *mut yaml_version_directive_t,
    pub tag_directives: yaml_tag_directives_range_t,
    pub implicit: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_document_end_t {
    pub implicit: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_alias_t {
    pub anchor: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_scalar_t {
    pub anchor: *mut yaml_char_t,
    pub tag: *mut yaml_char_t,
    pub value: *mut yaml_char_t,
    pub length: size_t,
    pub plain_implicit: c_int,
    pub quoted_implicit: c_int,
    pub style: yaml_scalar_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_sequence_start_t {
    pub anchor: *mut yaml_char_t,
    pub tag: *mut yaml_char_t,
    pub implicit: c_int,
    pub style: yaml_sequence_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_mapping_start_t {
    pub anchor: *mut yaml_char_t,
    pub tag: *mut yaml_char_t,
    pub implicit: c_int,
    pub style: yaml_mapping_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_event_data_t {
    pub stream_start: yaml_event_data_stream_start_t,
    pub document_start: yaml_event_data_document_start_t,
    pub document_end: yaml_event_data_document_end_t,
    pub alias: yaml_event_data_alias_t,
    pub scalar: yaml_event_data_scalar_t,
    pub sequence_start: yaml_event_data_sequence_start_t,
    pub mapping_start: yaml_event_data_mapping_start_t,
}

/// The event structure.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_t {
    /// The event type.
    pub type_: yaml_event_type_t,
    /// The event data.
    pub data: yaml_event_data_t,
    /// The beginning of the event.
    pub start_mark: yaml_mark_t,
    /// The end of the event.
    pub end_mark: yaml_mark_t,
}

/* ------------------------------------------------------------------ */
/* Nodes                                                               */
/* ------------------------------------------------------------------ */

pub const YAML_NULL_TAG: &[u8] = b"tag:yaml.org,2002:null\0";
pub const YAML_BOOL_TAG: &[u8] = b"tag:yaml.org,2002:bool\0";
pub const YAML_STR_TAG: &[u8] = b"tag:yaml.org,2002:str\0";
pub const YAML_INT_TAG: &[u8] = b"tag:yaml.org,2002:int\0";
pub const YAML_FLOAT_TAG: &[u8] = b"tag:yaml.org,2002:float\0";
pub const YAML_TIMESTAMP_TAG: &[u8] = b"tag:yaml.org,2002:timestamp\0";

pub const YAML_SEQ_TAG: &[u8] = b"tag:yaml.org,2002:seq\0";
pub const YAML_MAP_TAG: &[u8] = b"tag:yaml.org,2002:map\0";

pub const YAML_DEFAULT_SCALAR_TAG: &[u8] = YAML_STR_TAG;
pub const YAML_DEFAULT_SEQUENCE_TAG: &[u8] = YAML_SEQ_TAG;
pub const YAML_DEFAULT_MAPPING_TAG: &[u8] = YAML_MAP_TAG;

/// Node types.
pub type yaml_node_type_t = c_uint;
pub const YAML_NO_NODE: yaml_node_type_t = 0;
pub const YAML_SCALAR_NODE: yaml_node_type_t = 1;
pub const YAML_SEQUENCE_NODE: yaml_node_type_t = 2;
pub const YAML_MAPPING_NODE: yaml_node_type_t = 3;

/// An element of a sequence node.
pub type yaml_node_item_t = c_int;

/// An element of a mapping node.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_pair_t {
    /// The key of the element.
    pub key: c_int,
    /// The value of the element.
    pub value: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_data_scalar_t {
    pub value: *mut yaml_char_t,
    pub length: size_t,
    pub style: yaml_scalar_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_data_sequence_t {
    pub items: yaml_stack_t<yaml_node_item_t>,
    pub style: yaml_sequence_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_data_mapping_t {
    pub pairs: yaml_stack_t<yaml_node_pair_t>,
    pub style: yaml_mapping_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_node_data_t {
    pub scalar: yaml_node_data_scalar_t,
    pub sequence: yaml_node_data_sequence_t,
    pub mapping: yaml_node_data_mapping_t,
}

/// The node structure.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_t {
    /// The node type.
    pub type_: yaml_node_type_t,
    /// The node tag.
    pub tag: *mut yaml_char_t,
    /// The node data.
    pub data: yaml_node_data_t,
    /// The beginning of the node.
    pub start_mark: yaml_mark_t,
    /// The end of the node.
    pub end_mark: yaml_mark_t,
}

/// The document structure.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_document_t {
    /// The document nodes.
    pub nodes: yaml_stack_t<yaml_node_t>,
    /// The version directive.
    pub version_directive: *mut yaml_version_directive_t,
    /// The list of tag directives.
    pub tag_directives: yaml_tag_directives_range_t,
    /// Is the document start indicator implicit?
    pub start_implicit: c_int,
    /// Is the document end indicator implicit?
    pub end_implicit: c_int,
    /// The beginning of the document.
    pub start_mark: yaml_mark_t,
    /// The end of the document.
    pub end_mark: yaml_mark_t,
}

/* ------------------------------------------------------------------ */
/* Parser                                                              */
/* ------------------------------------------------------------------ */

/// The prototype of a read handler.
pub type yaml_read_handler_t = unsafe extern "C" fn(
    data: *mut c_void,
    buffer: *mut c_uchar,
    size: size_t,
    size_read: *mut size_t,
) -> c_int;

/// This structure holds information about a potential simple key.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_simple_key_t {
    /// Is a simple key possible?
    pub possible: c_int,
    /// Is a simple key required?
    pub required: c_int,
    /// The number of the token.
    pub token_number: size_t,
    /// The position mark.
    pub mark: yaml_mark_t,
}

/// The states of the parser.
pub type yaml_parser_state_t = c_uint;
pub const YAML_PARSE_STREAM_START_STATE: yaml_parser_state_t = 0;
pub const YAML_PARSE_IMPLICIT_DOCUMENT_START_STATE: yaml_parser_state_t = 1;
pub const YAML_PARSE_DOCUMENT_START_STATE: yaml_parser_state_t = 2;
pub const YAML_PARSE_DOCUMENT_CONTENT_STATE: yaml_parser_state_t = 3;
pub const YAML_PARSE_DOCUMENT_END_STATE: yaml_parser_state_t = 4;
pub const YAML_PARSE_BLOCK_NODE_STATE: yaml_parser_state_t = 5;
pub const YAML_PARSE_BLOCK_NODE_OR_INDENTLESS_SEQUENCE_STATE: yaml_parser_state_t = 6;
pub const YAML_PARSE_FLOW_NODE_STATE: yaml_parser_state_t = 7;
pub const YAML_PARSE_BLOCK_SEQUENCE_FIRST_ENTRY_STATE: yaml_parser_state_t = 8;
pub const YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE: yaml_parser_state_t = 9;
pub const YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE: yaml_parser_state_t = 10;
pub const YAML_PARSE_BLOCK_MAPPING_FIRST_KEY_STATE: yaml_parser_state_t = 11;
pub const YAML_PARSE_BLOCK_MAPPING_KEY_STATE: yaml_parser_state_t = 12;
pub const YAML_PARSE_BLOCK_MAPPING_VALUE_STATE: yaml_parser_state_t = 13;
pub const YAML_PARSE_FLOW_SEQUENCE_FIRST_ENTRY_STATE: yaml_parser_state_t = 14;
pub const YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE: yaml_parser_state_t = 15;
pub const YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_KEY_STATE: yaml_parser_state_t = 16;
pub const YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE: yaml_parser_state_t = 17;
pub const YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE: yaml_parser_state_t = 18;
pub const YAML_PARSE_FLOW_MAPPING_FIRST_KEY_STATE: yaml_parser_state_t = 19;
pub const YAML_PARSE_FLOW_MAPPING_KEY_STATE: yaml_parser_state_t = 20;
pub const YAML_PARSE_FLOW_MAPPING_VALUE_STATE: yaml_parser_state_t = 21;
pub const YAML_PARSE_FLOW_MAPPING_EMPTY_VALUE_STATE: yaml_parser_state_t = 22;
pub const YAML_PARSE_END_STATE: yaml_parser_state_t = 23;

/// This structure holds aliases data.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_alias_data_t {
    /// The anchor.
    pub anchor: *mut yaml_char_t,
    /// The node id.
    pub index: c_int,
    /// The anchor mark.
    pub mark: yaml_mark_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_input_string_t {
    /// The string start pointer.
    pub start: *const c_uchar,
    /// The string end pointer.
    pub end: *const c_uchar,
    /// The string current position.
    pub current: *const c_uchar,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_parser_input_t {
    /// String input data.
    pub string: yaml_parser_input_string_t,
    /// File input data.
    pub file: *mut FILE,
}

/// The parser structure.
#[repr(C)]
pub struct yaml_parser_t {
    /* Error handling */
    /// Error type.
    pub error: yaml_error_type_t,
    /// Error description.
    pub problem: *const libc_shim::c_char,
    /// The byte about which the problem occurred.
    pub problem_offset: size_t,
    /// The problematic value (-1 is none).
    pub problem_value: c_int,
    /// The problem position.
    pub problem_mark: yaml_mark_t,
    /// The error context.
    pub context: *const libc_shim::c_char,
    /// The context position.
    pub context_mark: yaml_mark_t,

    /* Reader stuff */
    /// Read handler.
    pub read_handler: Option<yaml_read_handler_t>,
    /// A pointer for passing to the read handler.
    pub read_handler_data: *mut c_void,
    /// Standard (string or file) input data.
    pub input: yaml_parser_input_t,
    /// EOF flag.
    pub eof: c_int,
    /// The working buffer.
    pub buffer: yaml_buffer_t,
    /// The number of unread characters in the buffer.
    pub unread: size_t,
    /// The raw buffer.
    pub raw_buffer: yaml_buffer_t,
    /// The input encoding.
    pub encoding: yaml_encoding_t,
    /// The offset of the current position (in bytes).
    pub offset: size_t,
    /// The mark of the current position.
    pub mark: yaml_mark_t,

    /* Scanner stuff */
    /// Have we started to scan the input stream?
    pub stream_start_produced: c_int,
    /// Have we reached the end of the input stream?
    pub stream_end_produced: c_int,
    /// The number of unclosed '[' and '{' indicators.
    pub flow_level: c_int,
    /// The tokens queue.
    pub tokens: yaml_queue_t<yaml_token_t>,
    /// The number of tokens fetched from the queue.
    pub tokens_parsed: size_t,
    /// Does the tokens queue contain a token ready for dequeueing.
    pub token_available: c_int,
    /// The indentation levels stack.
    pub indents: yaml_stack_t<c_int>,
    /// The current indentation level.
    pub indent: c_int,
    /// May a simple key occur at the current position?
    pub simple_key_allowed: c_int,
    /// The stack of simple keys.
    pub simple_keys: yaml_stack_t<yaml_simple_key_t>,

    /* Parser stuff */
    /// The parser states stack.
    pub states: yaml_stack_t<yaml_parser_state_t>,
    /// The current parser state.
    pub state: yaml_parser_state_t,
    /// The stack of marks.
    pub marks: yaml_stack_t<yaml_mark_t>,
    /// The list of TAG directives.
    pub tag_directives: yaml_stack_t<yaml_tag_directive_t>,

    /* Dumper stuff */
    /// The alias data.
    pub aliases: yaml_stack_t<yaml_alias_data_t>,
    /// The currently parsed document.
    pub document: *mut yaml_document_t,
}

/* ------------------------------------------------------------------ */
/* Emitter                                                             */
/* ------------------------------------------------------------------ */

/// The prototype of a write handler.
pub type yaml_write_handler_t =
    unsafe extern "C" fn(data: *mut c_void, buffer: *mut c_uchar, size: size_t) -> c_int;

/// The emitter states.
pub type yaml_emitter_state_t = c_uint;
pub const YAML_EMIT_STREAM_START_STATE: yaml_emitter_state_t = 0;
pub const YAML_EMIT_FIRST_DOCUMENT_START_STATE: yaml_emitter_state_t = 1;
pub const YAML_EMIT_DOCUMENT_START_STATE: yaml_emitter_state_t = 2;
pub const YAML_EMIT_DOCUMENT_CONTENT_STATE: yaml_emitter_state_t = 3;
pub const YAML_EMIT_DOCUMENT_END_STATE: yaml_emitter_state_t = 4;
pub const YAML_EMIT_FLOW_SEQUENCE_FIRST_ITEM_STATE: yaml_emitter_state_t = 5;
pub const YAML_EMIT_FLOW_SEQUENCE_ITEM_STATE: yaml_emitter_state_t = 6;
pub const YAML_EMIT_FLOW_MAPPING_FIRST_KEY_STATE: yaml_emitter_state_t = 7;
pub const YAML_EMIT_FLOW_MAPPING_KEY_STATE: yaml_emitter_state_t = 8;
pub const YAML_EMIT_FLOW_MAPPING_SIMPLE_VALUE_STATE: yaml_emitter_state_t = 9;
pub const YAML_EMIT_FLOW_MAPPING_VALUE_STATE: yaml_emitter_state_t = 10;
pub const YAML_EMIT_BLOCK_SEQUENCE_FIRST_ITEM_STATE: yaml_emitter_state_t = 11;
pub const YAML_EMIT_BLOCK_SEQUENCE_ITEM_STATE: yaml_emitter_state_t = 12;
pub const YAML_EMIT_BLOCK_MAPPING_FIRST_KEY_STATE: yaml_emitter_state_t = 13;
pub const YAML_EMIT_BLOCK_MAPPING_KEY_STATE: yaml_emitter_state_t = 14;
pub const YAML_EMIT_BLOCK_MAPPING_SIMPLE_VALUE_STATE: yaml_emitter_state_t = 15;
pub const YAML_EMIT_BLOCK_MAPPING_VALUE_STATE: yaml_emitter_state_t = 16;
pub const YAML_EMIT_END_STATE: yaml_emitter_state_t = 17;

/// This is needed for C++.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_anchors_t {
    /// The number of references.
    pub references: c_int,
    /// The anchor id.
    pub anchor: c_int,
    /// If the node has been emitted?
    pub serialized: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_output_string_t {
    /// The buffer pointer.
    pub buffer: *mut c_uchar,
    /// The buffer size.
    pub size: size_t,
    /// The number of written bytes.
    pub size_written: *mut size_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_emitter_output_t {
    /// String output data.
    pub string: yaml_emitter_output_string_t,
    /// File output data.
    pub file: *mut FILE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_anchor_data_t {
    /// The anchor value.
    pub anchor: *mut yaml_char_t,
    /// The anchor length.
    pub anchor_length: size_t,
    /// Is it an alias?
    pub alias: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_tag_data_t {
    /// The tag handle.
    pub handle: *mut yaml_char_t,
    /// The tag handle length.
    pub handle_length: size_t,
    /// The tag suffix.
    pub suffix: *mut yaml_char_t,
    /// The tag suffix length.
    pub suffix_length: size_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_scalar_data_t {
    /// The scalar value.
    pub value: *mut yaml_char_t,
    /// The scalar length.
    pub length: size_t,
    /// Does the scalar contain line breaks?
    pub multiline: c_int,
    /// Can the scalar be expressed in the flow plain style?
    pub flow_plain_allowed: c_int,
    /// Can the scalar be expressed in the block plain style?
    pub block_plain_allowed: c_int,
    /// Can the scalar be expressed in the single quoted style?
    pub single_quoted_allowed: c_int,
    /// Can the scalar be expressed in the literal or folded styles?
    pub block_allowed: c_int,
    /// The output style.
    pub style: yaml_scalar_style_t,
}

/// The emitter structure.
#[repr(C)]
pub struct yaml_emitter_t {
    /* Error handling */
    /// Error type.
    pub error: yaml_error_type_t,
    /// Error description.
    pub problem: *const libc_shim::c_char,

    /* Writer stuff */
    /// Write handler.
    pub write_handler: Option<yaml_write_handler_t>,
    /// A pointer for passing to the write handler.
    pub write_handler_data: *mut c_void,
    /// Standard (string or file) output data.
    pub output: yaml_emitter_output_t,
    /// The working buffer.
    pub buffer: yaml_buffer_t,
    /// The raw buffer.
    pub raw_buffer: yaml_buffer_t,
    /// The stream encoding.
    pub encoding: yaml_encoding_t,

    /* Emitter stuff */
    /// If the output is in the canonical style?
    pub canonical: c_int,
    /// The number of indentation spaces.
    pub best_indent: c_int,
    /// The preferred width of the output lines.
    pub best_width: c_int,
    /// Allow unescaped non-ASCII characters?
    pub unicode: c_int,
    /// The preferred line break.
    pub line_break: yaml_break_t,
    /// The stack of states.
    pub states: yaml_stack_t<yaml_emitter_state_t>,
    /// The current emitter state.
    pub state: yaml_emitter_state_t,
    /// The event queue.
    pub events: yaml_queue_t<yaml_event_t>,
    /// The stack of indentation levels.
    pub indents: yaml_stack_t<c_int>,
    /// The list of tag directives.
    pub tag_directives: yaml_stack_t<yaml_tag_directive_t>,
    /// The current indentation level.
    pub indent: c_int,
    /// The current flow level.
    pub flow_level: c_int,
    /// Is it the document root context?
    pub root_context: c_int,
    /// Is it a sequence context?
    pub sequence_context: c_int,
    /// Is it a mapping context?
    pub mapping_context: c_int,
    /// Is it a simple mapping key context?
    pub simple_key_context: c_int,
    /// The current line.
    pub line: c_int,
    /// The current column.
    pub column: c_int,
    /// If the last character was a whitespace?
    pub whitespace: c_int,
    /// If the last character was an indentation character (' ', '-', '?', ':')?
    pub indention: c_int,
    /// If an explicit document end is required?
    pub open_ended: c_int,
    /// Anchor analysis.
    pub anchor_data: yaml_emitter_anchor_data_t,
    /// Tag analysis.
    pub tag_data: yaml_emitter_tag_data_t,
    /// Scalar analysis.
    pub scalar_data: yaml_emitter_scalar_data_t,

    /* Dumper stuff */
    /// If the stream was already opened?
    pub opened: c_int,
    /// If the stream was already closed?
    pub closed: c_int,
    /// The information associated with the document nodes.
    pub anchors: *mut yaml_anchors_t,
    /// The last assigned anchor id.
    pub last_anchor_id: c_int,
    /// The currently emitted document.
    pub document: *mut yaml_document_t,
}

/* ------------------------------------------------------------------ */
/* ABI guards: these must match `c_src/include/yaml.h` compiled by a C */
/* compiler on the target platform.  Values below were measured with   */
/* gcc on x86-64 Linux.                                                */
/* ------------------------------------------------------------------ */

const _: () = {
    use core::mem::{align_of, size_of};
    assert!(size_of::<yaml_version_directive_t>() == 8);
    assert!(size_of::<yaml_tag_directive_t>() == 16);
    assert!(size_of::<yaml_mark_t>() == 24);
    assert!(size_of::<yaml_token_t>() == 80);
    assert!(size_of::<yaml_event_t>() == 104);
    assert!(size_of::<yaml_node_t>() == 96);
    assert!(size_of::<yaml_node_pair_t>() == 8);
    assert!(size_of::<yaml_document_t>() == 104);
    assert!(size_of::<yaml_simple_key_t>() == 40);
    assert!(size_of::<yaml_alias_data_t>() == 40);
    assert!(size_of::<yaml_anchors_t>() == 12);
    assert!(size_of::<yaml_parser_t>() == 480);
    assert!(size_of::<yaml_emitter_t>() == 432);
    assert!(align_of::<yaml_parser_t>() == 8);
    assert!(align_of::<yaml_emitter_t>() == 8);
    assert!(size_of::<Option<yaml_read_handler_t>>() == 8);
    assert!(size_of::<Option<yaml_write_handler_t>>() == 8);
};

macro_rules! assert_offset {
    ($ty:ty, $field:ident, $expected:expr) => {
        const _: () = {
            assert!(core::mem::offset_of!($ty, $field) == $expected);
        };
    };
}

assert_offset!(yaml_token_t, data, 8);
assert_offset!(yaml_token_t, start_mark, 32);
assert_offset!(yaml_event_t, data, 8);
assert_offset!(yaml_event_t, start_mark, 56);
assert_offset!(yaml_event_t, end_mark, 80);
assert_offset!(yaml_node_t, tag, 8);
assert_offset!(yaml_node_t, data, 16);
assert_offset!(yaml_node_t, start_mark, 48);
assert_offset!(yaml_document_t, version_directive, 24);
assert_offset!(yaml_document_t, tag_directives, 32);
assert_offset!(yaml_document_t, start_implicit, 48);
assert_offset!(yaml_document_t, start_mark, 56);

assert_offset!(yaml_parser_t, error, 0);
assert_offset!(yaml_parser_t, problem, 8);
assert_offset!(yaml_parser_t, problem_offset, 16);
assert_offset!(yaml_parser_t, problem_value, 24);
assert_offset!(yaml_parser_t, problem_mark, 32);
assert_offset!(yaml_parser_t, context, 56);
assert_offset!(yaml_parser_t, context_mark, 64);
assert_offset!(yaml_parser_t, read_handler, 88);
assert_offset!(yaml_parser_t, read_handler_data, 96);
assert_offset!(yaml_parser_t, input, 104);
assert_offset!(yaml_parser_t, eof, 128);
assert_offset!(yaml_parser_t, buffer, 136);
assert_offset!(yaml_parser_t, unread, 168);
assert_offset!(yaml_parser_t, raw_buffer, 176);
assert_offset!(yaml_parser_t, encoding, 208);
assert_offset!(yaml_parser_t, offset, 216);
assert_offset!(yaml_parser_t, mark, 224);
assert_offset!(yaml_parser_t, stream_start_produced, 248);
assert_offset!(yaml_parser_t, stream_end_produced, 252);
assert_offset!(yaml_parser_t, flow_level, 256);
assert_offset!(yaml_parser_t, tokens, 264);
assert_offset!(yaml_parser_t, tokens_parsed, 296);
assert_offset!(yaml_parser_t, token_available, 304);
assert_offset!(yaml_parser_t, indents, 312);
assert_offset!(yaml_parser_t, indent, 336);
assert_offset!(yaml_parser_t, simple_key_allowed, 340);
assert_offset!(yaml_parser_t, simple_keys, 344);
assert_offset!(yaml_parser_t, states, 368);
assert_offset!(yaml_parser_t, state, 392);
assert_offset!(yaml_parser_t, marks, 400);
assert_offset!(yaml_parser_t, tag_directives, 424);
assert_offset!(yaml_parser_t, aliases, 448);
assert_offset!(yaml_parser_t, document, 472);

assert_offset!(yaml_emitter_t, error, 0);
assert_offset!(yaml_emitter_t, problem, 8);
assert_offset!(yaml_emitter_t, write_handler, 16);
assert_offset!(yaml_emitter_t, write_handler_data, 24);
assert_offset!(yaml_emitter_t, output, 32);
assert_offset!(yaml_emitter_t, buffer, 56);
assert_offset!(yaml_emitter_t, raw_buffer, 88);
assert_offset!(yaml_emitter_t, encoding, 120);
assert_offset!(yaml_emitter_t, canonical, 124);
assert_offset!(yaml_emitter_t, best_indent, 128);
assert_offset!(yaml_emitter_t, best_width, 132);
assert_offset!(yaml_emitter_t, unicode, 136);
assert_offset!(yaml_emitter_t, line_break, 140);
assert_offset!(yaml_emitter_t, states, 144);
assert_offset!(yaml_emitter_t, state, 168);
assert_offset!(yaml_emitter_t, events, 176);
assert_offset!(yaml_emitter_t, indents, 208);
assert_offset!(yaml_emitter_t, tag_directives, 232);
assert_offset!(yaml_emitter_t, indent, 256);
assert_offset!(yaml_emitter_t, flow_level, 260);
assert_offset!(yaml_emitter_t, root_context, 264);
assert_offset!(yaml_emitter_t, sequence_context, 268);
assert_offset!(yaml_emitter_t, mapping_context, 272);
assert_offset!(yaml_emitter_t, simple_key_context, 276);
assert_offset!(yaml_emitter_t, line, 280);
assert_offset!(yaml_emitter_t, column, 284);
assert_offset!(yaml_emitter_t, whitespace, 288);
assert_offset!(yaml_emitter_t, indention, 292);
assert_offset!(yaml_emitter_t, open_ended, 296);
assert_offset!(yaml_emitter_t, anchor_data, 304);
assert_offset!(yaml_emitter_t, tag_data, 328);
assert_offset!(yaml_emitter_t, scalar_data, 360);
assert_offset!(yaml_emitter_t, opened, 400);
assert_offset!(yaml_emitter_t, closed, 404);
assert_offset!(yaml_emitter_t, anchors, 408);
assert_offset!(yaml_emitter_t, last_anchor_id, 416);
assert_offset!(yaml_emitter_t, document, 424);
