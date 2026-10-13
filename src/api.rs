//! Translation of `c_src/src/api.c` lines 164-582: parser/emitter creation,
//! destruction and configuration.

use crate::libc_shim::*;
use crate::private::*;
use crate::*;

/*
 * Create a new parser object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_initialize(parser: *mut yaml_parser_t) -> c_int {
    yaml_assert!(!parser.is_null()); /* Non-NULL parser object expected. */

    memset(parser as *mut c_void, 0, core::mem::size_of::<yaml_parser_t>());

    'error: {
        if !crate::BUFFER_INIT!(
            parser,
            (*parser).raw_buffer,
            crate::private::INPUT_RAW_BUFFER_SIZE
        ) {
            break 'error;
        }
        if !crate::BUFFER_INIT!(parser, (*parser).buffer, crate::private::INPUT_BUFFER_SIZE) {
            break 'error;
        }
        if !crate::QUEUE_INIT!(
            parser,
            (*parser).tokens,
            crate::private::INITIAL_QUEUE_SIZE
        ) {
            break 'error;
        }
        if !crate::STACK_INIT!(parser, (*parser).indents) {
            break 'error;
        }
        if !crate::STACK_INIT!(parser, (*parser).simple_keys) {
            break 'error;
        }
        if !crate::STACK_INIT!(parser, (*parser).states) {
            break 'error;
        }
        if !crate::STACK_INIT!(parser, (*parser).marks) {
            break 'error;
        }
        if !crate::STACK_INIT!(parser, (*parser).tag_directives) {
            break 'error;
        }

        return 1;
    }

    /* error: */

    crate::BUFFER_DEL!(parser, (*parser).raw_buffer);
    crate::BUFFER_DEL!(parser, (*parser).buffer);
    crate::QUEUE_DEL!(parser, (*parser).tokens);
    crate::STACK_DEL!(parser, (*parser).indents);
    crate::STACK_DEL!(parser, (*parser).simple_keys);
    crate::STACK_DEL!(parser, (*parser).states);
    crate::STACK_DEL!(parser, (*parser).marks);
    crate::STACK_DEL!(parser, (*parser).tag_directives);

    0
}

/*
 * Destroy a parser object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_delete(parser: *mut yaml_parser_t) {
    yaml_assert!(!parser.is_null()); /* Non-NULL parser object expected. */

    crate::BUFFER_DEL!(parser, (*parser).raw_buffer);
    crate::BUFFER_DEL!(parser, (*parser).buffer);
    while !crate::QUEUE_EMPTY!(parser, (*parser).tokens) {
        let mut __t = crate::DEQUEUE!(parser, (*parser).tokens);
        crate::api_objects::yaml_token_delete(&mut __t);
    }
    crate::QUEUE_DEL!(parser, (*parser).tokens);
    crate::STACK_DEL!(parser, (*parser).indents);
    crate::STACK_DEL!(parser, (*parser).simple_keys);
    crate::STACK_DEL!(parser, (*parser).states);
    crate::STACK_DEL!(parser, (*parser).marks);
    while !crate::STACK_EMPTY!(parser, (*parser).tag_directives) {
        let tag_directive: yaml_tag_directive_t = crate::POP!(parser, (*parser).tag_directives);
        crate::memory::yaml_free(tag_directive.handle as *mut c_void);
        crate::memory::yaml_free(tag_directive.prefix as *mut c_void);
    }
    crate::STACK_DEL!(parser, (*parser).tag_directives);

    memset(parser as *mut c_void, 0, core::mem::size_of::<yaml_parser_t>());
}

/*
 * String read handler.
 */

pub(crate) unsafe extern "C" fn yaml_string_read_handler(
    data: *mut c_void,
    buffer: *mut c_uchar,
    mut size: size_t,
    size_read: *mut size_t,
) -> c_int {
    let parser = data as *mut yaml_parser_t;

    if (*parser).input.string.current == (*parser).input.string.end {
        *size_read = 0;
        return 1;
    }

    if size
        > ((*parser)
            .input
            .string
            .end
            .offset_from((*parser).input.string.current)) as size_t
    {
        size = (*parser)
            .input
            .string
            .end
            .offset_from((*parser).input.string.current) as size_t;
    }

    memcpy(
        buffer as *mut c_void,
        (*parser).input.string.current as *const c_void,
        size,
    );
    (*parser).input.string.current = (*parser).input.string.current.add(size);
    *size_read = size;
    1
}

/*
 * File read handler.
 */

pub(crate) unsafe extern "C" fn yaml_file_read_handler(
    data: *mut c_void,
    buffer: *mut c_uchar,
    size: size_t,
    size_read: *mut size_t,
) -> c_int {
    let parser = data as *mut yaml_parser_t;

    *size_read = fread(buffer as *mut c_void, 1, size, (*parser).input.file);
    (ferror((*parser).input.file) == 0) as c_int
}

/*
 * Set a string input.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_set_input_string(
    parser: *mut yaml_parser_t,
    input: *const c_uchar,
    size: size_t,
) {
    yaml_assert!(!parser.is_null()); /* Non-NULL parser object expected. */
    yaml_assert!((*parser).read_handler.is_none()); /* You can set the source only once. */
    yaml_assert!(!input.is_null()); /* Non-NULL input string expected. */

    (*parser).read_handler = Some(yaml_string_read_handler);
    (*parser).read_handler_data = parser as *mut c_void;

    (*parser).input.string.start = input;
    (*parser).input.string.current = input;
    (*parser).input.string.end = input.add(size);
}

/*
 * Set a file input.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_set_input_file(parser: *mut yaml_parser_t, file: *mut FILE) {
    yaml_assert!(!parser.is_null()); /* Non-NULL parser object expected. */
    yaml_assert!((*parser).read_handler.is_none()); /* You can set the source only once. */
    yaml_assert!(!file.is_null()); /* Non-NULL file object expected. */

    (*parser).read_handler = Some(yaml_file_read_handler);
    (*parser).read_handler_data = parser as *mut c_void;

    (*parser).input.file = file;
}

/*
 * Set a generic input.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_set_input(
    parser: *mut yaml_parser_t,
    handler: Option<yaml_read_handler_t>,
    data: *mut c_void,
) {
    yaml_assert!(!parser.is_null()); /* Non-NULL parser object expected. */
    yaml_assert!((*parser).read_handler.is_none()); /* You can set the source only once. */
    yaml_assert!(handler.is_some()); /* Non-NULL read handler expected. */

    (*parser).read_handler = handler;
    (*parser).read_handler_data = data;
}

/*
 * Set the source encoding.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_parser_set_encoding(
    parser: *mut yaml_parser_t,
    encoding: yaml_encoding_t,
) {
    yaml_assert!(!parser.is_null()); /* Non-NULL parser object expected. */
    yaml_assert!((*parser).encoding == 0); /* Encoding is already set or detected. */

    (*parser).encoding = encoding;
}

/*
 * Create a new emitter object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_initialize(emitter: *mut yaml_emitter_t) -> c_int {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */

    memset(emitter as *mut c_void, 0, core::mem::size_of::<yaml_emitter_t>());

    'error: {
        if !crate::BUFFER_INIT!(emitter, (*emitter).buffer, crate::private::OUTPUT_BUFFER_SIZE) {
            break 'error;
        }
        if !crate::BUFFER_INIT!(
            emitter,
            (*emitter).raw_buffer,
            crate::private::OUTPUT_RAW_BUFFER_SIZE
        ) {
            break 'error;
        }
        if !crate::STACK_INIT!(emitter, (*emitter).states) {
            break 'error;
        }
        if !crate::QUEUE_INIT!(
            emitter,
            (*emitter).events,
            crate::private::INITIAL_QUEUE_SIZE
        ) {
            break 'error;
        }
        if !crate::STACK_INIT!(emitter, (*emitter).indents) {
            break 'error;
        }
        if !crate::STACK_INIT!(emitter, (*emitter).tag_directives) {
            break 'error;
        }

        return 1;
    }

    /* error: */

    crate::BUFFER_DEL!(emitter, (*emitter).buffer);
    crate::BUFFER_DEL!(emitter, (*emitter).raw_buffer);
    crate::STACK_DEL!(emitter, (*emitter).states);
    crate::QUEUE_DEL!(emitter, (*emitter).events);
    crate::STACK_DEL!(emitter, (*emitter).indents);
    crate::STACK_DEL!(emitter, (*emitter).tag_directives);

    0
}

/*
 * Destroy an emitter object.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_delete(emitter: *mut yaml_emitter_t) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */

    crate::BUFFER_DEL!(emitter, (*emitter).buffer);
    crate::BUFFER_DEL!(emitter, (*emitter).raw_buffer);
    crate::STACK_DEL!(emitter, (*emitter).states);
    while !crate::QUEUE_EMPTY!(emitter, (*emitter).events) {
        let mut __e = crate::DEQUEUE!(emitter, (*emitter).events);
        crate::api_objects::yaml_event_delete(&mut __e);
    }
    crate::QUEUE_DEL!(emitter, (*emitter).events);
    crate::STACK_DEL!(emitter, (*emitter).indents);
    /* NOTE: the C source passes the (unused) context argument `empty` here;
     * STACK_EMPTY ignores it, so the behaviour is unchanged. */
    while !crate::STACK_EMPTY!(empty, (*emitter).tag_directives) {
        let tag_directive: yaml_tag_directive_t = crate::POP!(emitter, (*emitter).tag_directives);
        crate::memory::yaml_free(tag_directive.handle as *mut c_void);
        crate::memory::yaml_free(tag_directive.prefix as *mut c_void);
    }
    crate::STACK_DEL!(emitter, (*emitter).tag_directives);
    crate::memory::yaml_free((*emitter).anchors as *mut c_void);

    memset(emitter as *mut c_void, 0, core::mem::size_of::<yaml_emitter_t>());
}

/*
 * String write handler.
 */

pub(crate) unsafe extern "C" fn yaml_string_write_handler(
    data: *mut c_void,
    buffer: *mut c_uchar,
    size: size_t,
) -> c_int {
    let emitter = data as *mut yaml_emitter_t;

    if (*emitter)
        .output
        .string
        .size
        .wrapping_sub(*(*emitter).output.string.size_written)
        < size
    {
        memcpy(
            (*emitter)
                .output
                .string
                .buffer
                .add(*(*emitter).output.string.size_written) as *mut c_void,
            buffer as *const c_void,
            (*emitter)
                .output
                .string
                .size
                .wrapping_sub(*(*emitter).output.string.size_written),
        );
        *(*emitter).output.string.size_written = (*emitter).output.string.size;
        return 0;
    }

    memcpy(
        (*emitter)
            .output
            .string
            .buffer
            .add(*(*emitter).output.string.size_written) as *mut c_void,
        buffer as *const c_void,
        size,
    );
    *(*emitter).output.string.size_written =
        (*(*emitter).output.string.size_written).wrapping_add(size);
    1
}

/*
 * File write handler.
 */

pub(crate) unsafe extern "C" fn yaml_file_write_handler(
    data: *mut c_void,
    buffer: *mut c_uchar,
    size: size_t,
) -> c_int {
    let emitter = data as *mut yaml_emitter_t;

    (fwrite(buffer as *const c_void, 1, size, (*emitter).output.file) == size) as c_int
}
/*
 * Set a string output.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_output_string(
    emitter: *mut yaml_emitter_t,
    output: *mut c_uchar,
    size: size_t,
    size_written: *mut size_t,
) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */
    yaml_assert!((*emitter).write_handler.is_none()); /* You can set the output only once. */
    yaml_assert!(!output.is_null()); /* Non-NULL output string expected. */

    (*emitter).write_handler = Some(yaml_string_write_handler);
    (*emitter).write_handler_data = emitter as *mut c_void;

    (*emitter).output.string.buffer = output;
    (*emitter).output.string.size = size;
    (*emitter).output.string.size_written = size_written;
    *size_written = 0;
}

/*
 * Set a file output.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_output_file(
    emitter: *mut yaml_emitter_t,
    file: *mut FILE,
) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */
    yaml_assert!((*emitter).write_handler.is_none()); /* You can set the output only once. */
    yaml_assert!(!file.is_null()); /* Non-NULL file object expected. */

    (*emitter).write_handler = Some(yaml_file_write_handler);
    (*emitter).write_handler_data = emitter as *mut c_void;

    (*emitter).output.file = file;
}

/*
 * Set a generic output handler.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_output(
    emitter: *mut yaml_emitter_t,
    handler: Option<yaml_write_handler_t>,
    data: *mut c_void,
) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */
    yaml_assert!((*emitter).write_handler.is_none()); /* You can set the output only once. */
    yaml_assert!(handler.is_some()); /* Non-NULL handler object expected. */

    (*emitter).write_handler = handler;
    (*emitter).write_handler_data = data;
}

/*
 * Set the output encoding.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_encoding(
    emitter: *mut yaml_emitter_t,
    encoding: yaml_encoding_t,
) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */
    yaml_assert!((*emitter).encoding == 0); /* You can set encoding only once. */

    (*emitter).encoding = encoding;
}

/*
 * Set the canonical output style.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_canonical(emitter: *mut yaml_emitter_t, canonical: c_int) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */

    (*emitter).canonical = (canonical != 0) as c_int;
}

/*
 * Set the indentation increment.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_indent(emitter: *mut yaml_emitter_t, indent: c_int) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */

    (*emitter).best_indent = if 1 < indent && indent < 10 { indent } else { 2 };
}

/*
 * Set the preferred line width.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_width(emitter: *mut yaml_emitter_t, width: c_int) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */

    (*emitter).best_width = if width >= 0 { width } else { -1 };
}

/*
 * Set if unescaped non-ASCII characters are allowed.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_unicode(emitter: *mut yaml_emitter_t, unicode: c_int) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */

    (*emitter).unicode = (unicode != 0) as c_int;
}

/*
 * Set the preferred line break character.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_emitter_set_break(
    emitter: *mut yaml_emitter_t,
    line_break: yaml_break_t,
) {
    yaml_assert!(!emitter.is_null()); /* Non-NULL emitter object expected. */

    (*emitter).line_break = line_break;
}
