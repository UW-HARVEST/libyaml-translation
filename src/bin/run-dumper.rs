//! Translation of `c_src/tests/run-dumper.c`.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_unsafe)]

#[path = "../lib.rs"]
mod yamllib;
use yamllib::*;

#[macro_use]
#[path = "../ctest.rs"]
mod ctest;

use crate::api::*;
use crate::api_document::*;
use crate::ctest::*;
use crate::dumper::*;
use crate::loader::*;
use crate::writer::*;

const BUFFER_SIZE: usize = 65536;
const MAX_DOCUMENTS: usize = 16;

unsafe fn copy_document(
    document_to: *mut yaml_document_t,
    document_from: *mut yaml_document_t,
) -> c_int {
    unsafe {
        let mut node: *mut yaml_node_t;
        let mut item: *mut yaml_node_item_t;
        let mut pair: *mut yaml_node_pair_t;

        if yaml_document_initialize(
            document_to,
            (*document_from).version_directive,
            (*document_from).tag_directives.start,
            (*document_from).tag_directives.end,
            (*document_from).start_implicit,
            (*document_from).end_implicit,
        ) == 0
        {
            return 0;
        }

        'error: {
            node = (*document_from).nodes.start;
            while node < (*document_from).nodes.top {
                match (*node).type_ {
                    YAML_SCALAR_NODE => {
                        if yaml_document_add_scalar(
                            document_to,
                            (*node).tag,
                            (*node).data.scalar.value,
                            (*node).data.scalar.length as c_int,
                            (*node).data.scalar.style,
                        ) == 0
                        {
                            break 'error;
                        }
                    }
                    YAML_SEQUENCE_NODE => {
                        if yaml_document_add_sequence(
                            document_to,
                            (*node).tag,
                            (*node).data.sequence.style,
                        ) == 0
                        {
                            break 'error;
                        }
                    }
                    YAML_MAPPING_NODE => {
                        if yaml_document_add_mapping(
                            document_to,
                            (*node).tag,
                            (*node).data.mapping.style,
                        ) == 0
                        {
                            break 'error;
                        }
                    }
                    _ => {
                        c_assert!(0);
                    }
                }
                node = node.offset(1);
            }

            node = (*document_from).nodes.start;
            while node < (*document_from).nodes.top {
                match (*node).type_ {
                    YAML_SEQUENCE_NODE => {
                        item = (*node).data.sequence.items.start;
                        while item < (*node).data.sequence.items.top {
                            if yaml_document_append_sequence_item(
                                document_to,
                                (node.offset_from((*document_from).nodes.start) + 1) as c_int,
                                *item,
                            ) == 0
                            {
                                break 'error;
                            }
                            item = item.offset(1);
                        }
                    }
                    YAML_MAPPING_NODE => {
                        pair = (*node).data.mapping.pairs.start;
                        while pair < (*node).data.mapping.pairs.top {
                            if yaml_document_append_mapping_pair(
                                document_to,
                                (node.offset_from((*document_from).nodes.start) + 1) as c_int,
                                (*pair).key,
                                (*pair).value,
                            ) == 0
                            {
                                break 'error;
                            }
                            pair = pair.offset(1);
                        }
                    }
                    _ => {}
                }
                node = node.offset(1);
            }
            return 1;
        }

        /* error: */
        yaml_document_delete(document_to);
        0
    }
}

unsafe fn compare_nodes(
    document1: *mut yaml_document_t,
    index1: c_int,
    document2: *mut yaml_document_t,
    index2: c_int,
    mut level: c_int,
) -> c_int {
    unsafe {
        let mut k: isize;
        let node1: *mut yaml_node_t;
        let node2: *mut yaml_node_t;
        let level_old = level;
        level += 1;
        if level_old > 1000 {
            return 0;
        }
        node1 = yaml_document_get_node(document1, index1);
        node2 = yaml_document_get_node(document2, index2);

        c_assert!(node1);
        c_assert!(node2);

        if (*node1).type_ != (*node2).type_ {
            return 0;
        }

        if strcmp((*node1).tag as *const c_char, (*node2).tag as *const c_char) != 0 {
            return 0;
        }

        match (*node1).type_ {
            YAML_SCALAR_NODE => {
                if (*node1).data.scalar.length != (*node2).data.scalar.length {
                    return 0;
                }
                if strncmp(
                    (*node1).data.scalar.value as *const c_char,
                    (*node2).data.scalar.value as *const c_char,
                    (*node1).data.scalar.length,
                ) != 0
                {
                    return 0;
                }
            }
            YAML_SEQUENCE_NODE => {
                if (*node1)
                    .data
                    .sequence
                    .items
                    .top
                    .offset_from((*node1).data.sequence.items.start)
                    != (*node2)
                        .data
                        .sequence
                        .items
                        .top
                        .offset_from((*node2).data.sequence.items.start)
                {
                    return 0;
                }
                k = 0;
                while k
                    < (*node1)
                        .data
                        .sequence
                        .items
                        .top
                        .offset_from((*node1).data.sequence.items.start)
                {
                    if compare_nodes(
                        document1,
                        *(*node1).data.sequence.items.start.offset(k),
                        document2,
                        *(*node2).data.sequence.items.start.offset(k),
                        level,
                    ) == 0
                    {
                        return 0;
                    }
                    k += 1;
                }
            }
            YAML_MAPPING_NODE => {
                if (*node1)
                    .data
                    .mapping
                    .pairs
                    .top
                    .offset_from((*node1).data.mapping.pairs.start)
                    != (*node2)
                        .data
                        .mapping
                        .pairs
                        .top
                        .offset_from((*node2).data.mapping.pairs.start)
                {
                    return 0;
                }
                k = 0;
                while k
                    < (*node1)
                        .data
                        .mapping
                        .pairs
                        .top
                        .offset_from((*node1).data.mapping.pairs.start)
                {
                    if compare_nodes(
                        document1,
                        (*(*node1).data.mapping.pairs.start.offset(k)).key,
                        document2,
                        (*(*node2).data.mapping.pairs.start.offset(k)).key,
                        level,
                    ) == 0
                    {
                        return 0;
                    }
                    if compare_nodes(
                        document1,
                        (*(*node1).data.mapping.pairs.start.offset(k)).value,
                        document2,
                        (*(*node2).data.mapping.pairs.start.offset(k)).value,
                        level,
                    ) == 0
                    {
                        return 0;
                    }
                    k += 1;
                }
            }
            _ => {
                c_assert!(0);
            }
        }
        1
    }
}

unsafe fn compare_documents(
    document1: *mut yaml_document_t,
    document2: *mut yaml_document_t,
) -> c_int {
    unsafe {
        let mut k: isize;

        if (!(*document1).version_directive.is_null() && (*document2).version_directive.is_null())
            || ((*document1).version_directive.is_null()
                && !(*document2).version_directive.is_null())
            || (!(*document1).version_directive.is_null()
                && !(*document2).version_directive.is_null()
                && ((*(*document1).version_directive).major
                    != (*(*document2).version_directive).major
                    || (*(*document1).version_directive).minor
                        != (*(*document2).version_directive).minor))
        {
            return 0;
        }

        if (*document1)
            .tag_directives
            .end
            .offset_from((*document1).tag_directives.start)
            != (*document2)
                .tag_directives
                .end
                .offset_from((*document2).tag_directives.start)
        {
            return 0;
        }
        k = 0;
        while k
            < (*document1)
                .tag_directives
                .end
                .offset_from((*document1).tag_directives.start)
        {
            if (strcmp(
                (*(*document1).tag_directives.start.offset(k)).handle as *const c_char,
                (*(*document2).tag_directives.start.offset(k)).handle as *const c_char,
            ) != 0)
                || (strcmp(
                    (*(*document1).tag_directives.start.offset(k)).prefix as *const c_char,
                    (*(*document2).tag_directives.start.offset(k)).prefix as *const c_char,
                ) != 0)
            {
                return 0;
            }
            k += 1;
        }

        if (*document1).nodes.top.offset_from((*document1).nodes.start)
            != (*document2).nodes.top.offset_from((*document2).nodes.start)
        {
            return 0;
        }

        if (*document1).nodes.top != (*document1).nodes.start {
            if compare_nodes(document1, 1, document2, 1, 0) == 0 {
                return 0;
            }
        }

        1
    }
}

unsafe fn print_output(
    name: *mut c_char,
    buffer: *mut c_uchar,
    size: size_t,
    count: c_int,
) -> c_int {
    unsafe {
        let file: *mut FILE;
        let mut data: [c_char; BUFFER_SIZE] = [0; BUFFER_SIZE];
        let mut data_size: size_t = 1;
        let mut total_size: size_t = 0;
        if count >= 0 {
            printf(
                cstr!("FAILED (at the document #%d)\nSOURCE:\n"),
                count + 1,
            );
        }
        file = fopen(name, cstr!("rb"));
        c_assert!(file);
        while data_size > 0 {
            data_size = fread(data.as_mut_ptr() as *mut c_void, 1, BUFFER_SIZE, file);
            c_assert!(ferror(file) == 0);
            if data_size == 0 {
                break;
            }
            c_assert!(fwrite(data.as_ptr() as *const c_void, 1, data_size, stdout) == data_size);
            total_size += data_size;
            if feof(file) != 0 {
                break;
            }
        }
        fclose(file);
        printf(cstr!("#### (length: %ld)\n"), total_size as c_long);
        printf(
            cstr!("OUTPUT:\n%s#### (length: %ld)\n"),
            buffer as *const c_char,
            size as c_long,
        );
        0
    }
}

unsafe fn c_main(mut argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        let mut number: c_int;
        let mut canonical: c_int = 0;
        let mut unicode: c_int = 0;

        number = 1;
        while number < argc {
            if strcmp(*argv.offset(number as isize), cstr!("-c")) == 0 {
                canonical = 1;
            } else if strcmp(*argv.offset(number as isize), cstr!("-u")) == 0 {
                unicode = 1;
            } else if *(*argv.offset(number as isize)).offset(0) == b'-' as c_char {
                printf(
                    cstr!("Unknown option: '%s'\n"),
                    *argv.offset(number as isize),
                );
                return 0;
            }
            if *(*argv.offset(number as isize)).offset(0) == b'-' as c_char {
                if number < argc - 1 {
                    memmove(
                        argv.offset(number as isize) as *mut c_void,
                        argv.offset(number as isize + 1) as *const c_void,
                        (argc - number - 1) as size_t * core::mem::size_of::<*mut c_char>(),
                    );
                }
                argc -= 1;
            } else {
                number += 1;
            }
        }

        if argc < 2 {
            printf(
                cstr!("Usage: %s [-c] [-u] file1.yaml ...\n"),
                *argv.offset(0),
            );
            return 0;
        }

        number = 1;
        while number < argc {
            let file: *mut FILE;
            let mut parser: yaml_parser_t = core::mem::zeroed();
            let mut emitter: yaml_emitter_t = core::mem::zeroed();

            let mut document: yaml_document_t = core::mem::zeroed();
            let mut buffer: [c_uchar; BUFFER_SIZE + 1] = [0; BUFFER_SIZE + 1];
            let mut written: size_t = 0;
            let mut documents: [yaml_document_t; MAX_DOCUMENTS] = core::mem::zeroed();
            let mut document_number: size_t = 0;
            let mut done: c_int = 0;
            let mut count: c_int = 0;
            let mut error: c_int = 0;
            let mut k: c_int;
            memset(buffer.as_mut_ptr() as *mut c_void, 0, BUFFER_SIZE + 1);
            memset(
                documents.as_mut_ptr() as *mut c_void,
                0,
                MAX_DOCUMENTS * core::mem::size_of::<yaml_document_t>(),
            );

            printf(
                cstr!("[%d] Loading, dumping, and loading again '%s': "),
                number,
                *argv.offset(number as isize),
            );
            fflush(stdout);

            file = fopen(*argv.offset(number as isize), cstr!("rb"));
            c_assert!(file);

            c_assert!(yaml_parser_initialize(&mut parser));
            yaml_parser_set_input_file(&mut parser, file);
            c_assert!(yaml_emitter_initialize(&mut emitter));
            if canonical != 0 {
                yaml_emitter_set_canonical(&mut emitter, 1);
            }
            if unicode != 0 {
                yaml_emitter_set_unicode(&mut emitter, 1);
            }
            yaml_emitter_set_output_string(
                &mut emitter,
                buffer.as_mut_ptr(),
                BUFFER_SIZE,
                &mut written,
            );
            yaml_emitter_open(&mut emitter);

            while done == 0 {
                if yaml_parser_load(&mut parser, &mut document) == 0 {
                    error = 1;
                    break;
                }

                done = yaml_document_get_root_node(&mut document).is_null() as c_int;
                if done == 0 {
                    c_assert!(document_number < MAX_DOCUMENTS);
                    let index = document_number;
                    document_number += 1;
                    c_assert!(copy_document(
                        documents.as_mut_ptr().offset(index as isize),
                        &mut document
                    ));
                    c_assert!(
                        yaml_emitter_dump(&mut emitter, &mut document) != 0
                            || (yaml_emitter_flush(&mut emitter) != 0
                                && print_output(
                                    *argv.offset(number as isize),
                                    buffer.as_mut_ptr(),
                                    written,
                                    count
                                ) != 0)
                    );
                    count += 1;
                } else {
                    yaml_document_delete(&mut document);
                }
            }

            yaml_parser_delete(&mut parser);
            c_assert!(fclose(file) == 0);
            yaml_emitter_close(&mut emitter);
            yaml_emitter_delete(&mut emitter);

            if error == 0 {
                done = 0;
                count = done;
                c_assert!(yaml_parser_initialize(&mut parser));
                yaml_parser_set_input_string(&mut parser, buffer.as_ptr(), written);

                while done == 0 {
                    c_assert!(
                        yaml_parser_load(&mut parser, &mut document) != 0
                            || print_output(
                                *argv.offset(number as isize),
                                buffer.as_mut_ptr(),
                                written,
                                count
                            ) != 0
                    );
                    done = yaml_document_get_root_node(&mut document).is_null() as c_int;
                    if done == 0 {
                        c_assert!(
                            compare_documents(
                                documents.as_mut_ptr().offset(count as isize),
                                &mut document
                            ) != 0
                                || print_output(
                                    *argv.offset(number as isize),
                                    buffer.as_mut_ptr(),
                                    written,
                                    count
                                ) != 0
                        );
                        count += 1;
                    }
                    yaml_document_delete(&mut document);
                }
                yaml_parser_delete(&mut parser);
            }

            k = 0;
            while (k as size_t) < document_number {
                yaml_document_delete(documents.as_mut_ptr().offset(k as isize));
                k += 1;
            }

            printf(cstr!("PASSED (length: %ld)\n"), written as c_long);
            print_output(
                *argv.offset(number as isize),
                buffer.as_mut_ptr(),
                written,
                -1,
            );

            number += 1;
        }

        0
    }
}

fn main() {
    ctest::run(c_main)
}
