//! Translation of `c_src/tests/run-emitter.c`.

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
use crate::api_objects::*;
use crate::ctest::*;
use crate::emitter::*;
use crate::parser::*;
use crate::writer::*;

const BUFFER_SIZE: usize = 65536;
const MAX_EVENTS: usize = 1024;

unsafe fn copy_event(event_to: *mut yaml_event_t, event_from: *mut yaml_event_t) -> c_int {
    unsafe {
        match (*event_from).type_ {
            YAML_STREAM_START_EVENT => {
                return yaml_stream_start_event_initialize(
                    event_to,
                    (*event_from).data.stream_start.encoding,
                );
            }

            YAML_STREAM_END_EVENT => {
                return yaml_stream_end_event_initialize(event_to);
            }

            YAML_DOCUMENT_START_EVENT => {
                return yaml_document_start_event_initialize(
                    event_to,
                    (*event_from).data.document_start.version_directive,
                    (*event_from).data.document_start.tag_directives.start,
                    (*event_from).data.document_start.tag_directives.end,
                    (*event_from).data.document_start.implicit,
                );
            }

            YAML_DOCUMENT_END_EVENT => {
                return yaml_document_end_event_initialize(
                    event_to,
                    (*event_from).data.document_end.implicit,
                );
            }

            YAML_ALIAS_EVENT => {
                return yaml_alias_event_initialize(event_to, (*event_from).data.alias.anchor);
            }

            YAML_SCALAR_EVENT => {
                return yaml_scalar_event_initialize(
                    event_to,
                    (*event_from).data.scalar.anchor,
                    (*event_from).data.scalar.tag,
                    (*event_from).data.scalar.value,
                    (*event_from).data.scalar.length as c_int,
                    (*event_from).data.scalar.plain_implicit,
                    (*event_from).data.scalar.quoted_implicit,
                    (*event_from).data.scalar.style,
                );
            }

            YAML_SEQUENCE_START_EVENT => {
                return yaml_sequence_start_event_initialize(
                    event_to,
                    (*event_from).data.sequence_start.anchor,
                    (*event_from).data.sequence_start.tag,
                    (*event_from).data.sequence_start.implicit,
                    (*event_from).data.sequence_start.style,
                );
            }

            YAML_SEQUENCE_END_EVENT => {
                return yaml_sequence_end_event_initialize(event_to);
            }

            YAML_MAPPING_START_EVENT => {
                return yaml_mapping_start_event_initialize(
                    event_to,
                    (*event_from).data.mapping_start.anchor,
                    (*event_from).data.mapping_start.tag,
                    (*event_from).data.mapping_start.implicit,
                    (*event_from).data.mapping_start.style,
                );
            }

            YAML_MAPPING_END_EVENT => {
                return yaml_mapping_end_event_initialize(event_to);
            }

            _ => {
                c_assert!(1);
            }
        }

        return 0;
    }
}

unsafe fn compare_events(event1: *mut yaml_event_t, event2: *mut yaml_event_t) -> c_int {
    unsafe {
        let mut k: isize;

        if (*event1).type_ != (*event2).type_ {
            return 0;
        }

        match (*event1).type_ {
            YAML_STREAM_START_EVENT => {
                return 1;
                /* return (event1->data.stream_start.encoding ==
                        event2->data.stream_start.encoding); */
            }

            YAML_DOCUMENT_START_EVENT => {
                let vd1 = (*event1).data.document_start.version_directive;
                let vd2 = (*event2).data.document_start.version_directive;
                if (!vd1.is_null() && vd2.is_null())
                    || (vd1.is_null() && !vd2.is_null())
                    || (!vd1.is_null()
                        && !vd2.is_null()
                        && ((*vd1).major != (*vd2).major || (*vd1).minor != (*vd2).minor))
                {
                    return 0;
                }
                let start1 = (*event1).data.document_start.tag_directives.start;
                let end1 = (*event1).data.document_start.tag_directives.end;
                let start2 = (*event2).data.document_start.tag_directives.start;
                let end2 = (*event2).data.document_start.tag_directives.end;
                let elem = core::mem::size_of::<yaml_tag_directive_t>() as isize;
                let n1 = (end1 as isize - start1 as isize) / elem;
                let n2 = (end2 as isize - start2 as isize) / elem;
                if n1 != n2 {
                    return 0;
                }
                k = 0;
                while k < n1 {
                    if (strcmp(
                        (*start1.offset(k)).handle as *const c_char,
                        (*start2.offset(k)).handle as *const c_char,
                    ) != 0)
                        || (strcmp(
                            (*start1.offset(k)).prefix as *const c_char,
                            (*start2.offset(k)).prefix as *const c_char,
                        ) != 0)
                    {
                        return 0;
                    }
                    k += 1;
                }
                /* if (event1->data.document_start.implicit != event2->data.document_start.implicit)
                    return 0; */
                return 1;
            }

            YAML_DOCUMENT_END_EVENT => {
                return 1;
                /* return (event1->data.document_end.implicit ==
                        event2->data.document_end.implicit); */
            }

            YAML_ALIAS_EVENT => {
                return (strcmp(
                    (*event1).data.alias.anchor as *const c_char,
                    (*event2).data.alias.anchor as *const c_char,
                ) == 0) as c_int;
            }

            YAML_SCALAR_EVENT => {
                let anchor1 = (*event1).data.scalar.anchor;
                let anchor2 = (*event2).data.scalar.anchor;
                if (!anchor1.is_null() && anchor2.is_null())
                    || (anchor1.is_null() && !anchor2.is_null())
                    || (!anchor1.is_null()
                        && !anchor2.is_null()
                        && strcmp(anchor1 as *const c_char, anchor2 as *const c_char) != 0)
                {
                    return 0;
                }
                let tag1 = (*event1).data.scalar.tag;
                let tag2 = (*event2).data.scalar.tag;
                if (!tag1.is_null()
                    && tag2.is_null()
                    && strcmp(tag1 as *const c_char, cstr!("!")) != 0)
                    || (tag1.is_null()
                        && !tag2.is_null()
                        && strcmp(tag2 as *const c_char, cstr!("!")) != 0)
                    || (!tag1.is_null()
                        && !tag2.is_null()
                        && strcmp(tag1 as *const c_char, tag2 as *const c_char) != 0)
                {
                    return 0;
                }
                if ((*event1).data.scalar.length != (*event2).data.scalar.length)
                    || memcmp(
                        (*event1).data.scalar.value as *const c_void,
                        (*event2).data.scalar.value as *const c_void,
                        (*event1).data.scalar.length,
                    ) != 0
                {
                    return 0;
                }
                if ((*event1).data.scalar.plain_implicit != (*event2).data.scalar.plain_implicit)
                    || ((*event1).data.scalar.quoted_implicit
                        != (*event2).data.scalar.quoted_implicit)
                /* || (event2->data.scalar.style != event2->data.scalar.style) */
                {
                    return 0;
                }
                return 1;
            }

            YAML_SEQUENCE_START_EVENT => {
                let anchor1 = (*event1).data.sequence_start.anchor;
                let anchor2 = (*event2).data.sequence_start.anchor;
                if (!anchor1.is_null() && anchor2.is_null())
                    || (anchor1.is_null() && !anchor2.is_null())
                    || (!anchor1.is_null()
                        && !anchor2.is_null()
                        && strcmp(anchor1 as *const c_char, anchor2 as *const c_char) != 0)
                {
                    return 0;
                }
                let tag1 = (*event1).data.sequence_start.tag;
                let tag2 = (*event2).data.sequence_start.tag;
                if (!tag1.is_null() && tag2.is_null())
                    || (tag1.is_null() && !tag2.is_null())
                    || (!tag1.is_null()
                        && !tag2.is_null()
                        && strcmp(tag1 as *const c_char, tag2 as *const c_char) != 0)
                {
                    return 0;
                }
                if ((*event1).data.sequence_start.implicit
                    != (*event2).data.sequence_start.implicit)
                /* || (event2->data.sequence_start.style != event2->data.sequence_start.style) */
                {
                    return 0;
                }
                return 1;
            }

            YAML_MAPPING_START_EVENT => {
                let anchor1 = (*event1).data.mapping_start.anchor;
                let anchor2 = (*event2).data.mapping_start.anchor;
                if (!anchor1.is_null() && anchor2.is_null())
                    || (anchor1.is_null() && !anchor2.is_null())
                    || (!anchor1.is_null()
                        && !anchor2.is_null()
                        && strcmp(anchor1 as *const c_char, anchor2 as *const c_char) != 0)
                {
                    return 0;
                }
                let tag1 = (*event1).data.mapping_start.tag;
                let tag2 = (*event2).data.mapping_start.tag;
                if (!tag1.is_null() && tag2.is_null())
                    || (tag1.is_null() && !tag2.is_null())
                    || (!tag1.is_null()
                        && !tag2.is_null()
                        && strcmp(tag1 as *const c_char, tag2 as *const c_char) != 0)
                {
                    return 0;
                }
                if ((*event1).data.mapping_start.implicit != (*event2).data.mapping_start.implicit)
                /* || (event2->data.mapping_start.style != event2->data.mapping_start.style) */
                {
                    return 0;
                }
                return 1;
            }

            _ => {
                return 1;
            }
        }
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
            printf(cstr!("FAILED (at the event #%d)\nSOURCE:\n"), count + 1);
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
        return 0;
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
            let mut event: yaml_event_t = core::mem::zeroed();
            let mut buffer: [c_uchar; BUFFER_SIZE + 1] = [0; BUFFER_SIZE + 1];
            let mut written: size_t = 0;
            let mut events: [yaml_event_t; MAX_EVENTS] = core::mem::zeroed();
            let mut event_number: size_t = 0;
            let mut done: c_int = 0;
            let mut count: c_int = 0;
            let mut error: c_int = 0;
            let mut k: c_int;
            memset(buffer.as_mut_ptr() as *mut c_void, 0, BUFFER_SIZE + 1);
            memset(
                events.as_mut_ptr() as *mut c_void,
                0,
                MAX_EVENTS * core::mem::size_of::<yaml_event_t>(),
            );

            printf(
                cstr!("[%d] Parsing, emitting, and parsing again '%s': "),
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

            while done == 0 {
                if yaml_parser_parse(&mut parser, &mut event) == 0 {
                    error = 1;
                    break;
                }

                done = ((event).type_ == YAML_STREAM_END_EVENT) as c_int;
                c_assert!(event_number < MAX_EVENTS);
                c_assert!({
                    let __k = event_number;
                    event_number += 1;
                    copy_event(events.as_mut_ptr().add(__k), &mut event)
                });
                c_assert!(
                    yaml_emitter_emit(&mut emitter, &mut event) != 0
                        || print_output(
                            *argv.offset(number as isize),
                            buffer.as_mut_ptr(),
                            written,
                            count
                        ) != 0
                );
                count += 1;
            }

            yaml_parser_delete(&mut parser);
            c_assert!(fclose(file) == 0);
            yaml_emitter_delete(&mut emitter);

            if error == 0 {
                count = 0;
                done = 0;
                c_assert!(yaml_parser_initialize(&mut parser));
                yaml_parser_set_input_string(&mut parser, buffer.as_ptr(), written);

                while done == 0 {
                    c_assert!(
                        yaml_parser_parse(&mut parser, &mut event) != 0
                            || print_output(
                                *argv.offset(number as isize),
                                buffer.as_mut_ptr(),
                                written,
                                count
                            ) != 0
                    );
                    done = ((event).type_ == YAML_STREAM_END_EVENT) as c_int;
                    c_assert!(
                        compare_events(events.as_mut_ptr().offset(count as isize), &mut event) != 0
                            || print_output(
                                *argv.offset(number as isize),
                                buffer.as_mut_ptr(),
                                written,
                                count
                            ) != 0
                    );
                    yaml_event_delete(&mut event);
                    count += 1;
                }
                yaml_parser_delete(&mut parser);
            }

            k = 0;
            while (k as size_t) < event_number {
                yaml_event_delete(events.as_mut_ptr().offset(k as isize));
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

        return 0;
    }
}

fn main() {
    ctest::run(c_main)
}
