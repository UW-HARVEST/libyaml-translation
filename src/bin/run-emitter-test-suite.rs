//! Translation of `c_src/tests/run-emitter-test-suite.c`.

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
use crate::memory::yaml_malloc;

unsafe fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        let mut input: *mut FILE = core::ptr::null_mut();
        let mut emitter: yaml_emitter_t = core::mem::zeroed();
        let mut event: yaml_event_t = core::mem::zeroed();
        let mut version_directive: *mut yaml_version_directive_t = core::ptr::null_mut();

        let mut canonical: c_int = 0;
        let mut unicode: c_int = 0;
        let mut line: [c_char; 1024] = [0; 1024];
        let mut foundfile: c_int = 0;
        let mut i: c_int = 0;
        let mut minor: c_int = 0;
        let mut flow: c_int = -1; /* default no flow style collections */

        i = 1;
        while i < argc {
            if strncmp(*argv.offset(i as isize), cstr!("--help"), 6) == 0 {
                return usage(0);
            }
            if strncmp(*argv.offset(i as isize), cstr!("-h"), 2) == 0 {
                return usage(0);
            }
            if strncmp(*argv.offset(i as isize), cstr!("--flow"), 6) == 0 {
                if i + 1 == argc {
                    return usage(1);
                }
                i += 1;
                if strncmp(*argv.offset(i as isize), cstr!("keep"), 4) == 0 {
                    flow = 0;
                } else if strncmp(*argv.offset(i as isize), cstr!("on"), 2) == 0 {
                    flow = 1;
                } else if strncmp(*argv.offset(i as isize), cstr!("off"), 3) == 0 {
                    flow = -1;
                } else {
                    return usage(1);
                }
            } else if strncmp(*argv.offset(i as isize), cstr!("--directive"), 11) == 0 {
                if i + 1 == argc {
                    return usage(1);
                }
                i += 1;
                if strncmp(*argv.offset(i as isize), cstr!("1.1"), 3) == 0 {
                    minor = 1;
                } else if strncmp(*argv.offset(i as isize), cstr!("1.2"), 3) == 0 {
                    minor = 2;
                } else {
                    return usage(1);
                }
            } else if foundfile == 0 {
                input = fopen(*argv.offset(i as isize), cstr!("rb"));
                foundfile = 1;
            }

            i += 1;
        }
        if minor != 0 {
            version_directive = yaml_malloc(core::mem::size_of::<yaml_version_directive_t>())
                as *mut yaml_version_directive_t;
            (*version_directive).major = 1;
            (*version_directive).minor = minor;
        }
        if foundfile == 0 {
            input = stdin;
        }

        c_assert!(input);

        if yaml_emitter_initialize(&mut emitter) == 0 {
            fprintf(stderr, cstr!("Could not initialize the emitter object\n"));
            return 1;
        }
        yaml_emitter_set_output_file(&mut emitter, stdout);
        yaml_emitter_set_canonical(&mut emitter, canonical);
        yaml_emitter_set_unicode(&mut emitter, unicode);

        'emitter_error: {
            'event_error: {
                while get_line(input, line.as_mut_ptr()) != 0 {
                    let ok: c_int;
                    let mut anchor: [c_char; 256] = [0; 256];
                    let mut tag: [c_char; 256] = [0; 256];
                    let mut implicit: c_int;
                    let mut style: c_int;

                    if strncmp(line.as_ptr(), cstr!("+STR"), 4) == 0 {
                        ok = yaml_stream_start_event_initialize(&mut event, YAML_UTF8_ENCODING);
                    } else if strncmp(line.as_ptr(), cstr!("-STR"), 4) == 0 {
                        ok = yaml_stream_end_event_initialize(&mut event);
                    } else if strncmp(line.as_ptr(), cstr!("+DOC"), 4) == 0 {
                        implicit =
                            (strncmp(line.as_ptr().offset(4), cstr!(" ---"), 4) != 0) as c_int;
                        ok = yaml_document_start_event_initialize(
                            &mut event,
                            version_directive,
                            core::ptr::null_mut(),
                            core::ptr::null_mut(),
                            implicit,
                        );
                    } else if strncmp(line.as_ptr(), cstr!("-DOC"), 4) == 0 {
                        implicit =
                            (strncmp(line.as_ptr().offset(4), cstr!(" ..."), 4) != 0) as c_int;
                        ok = yaml_document_end_event_initialize(&mut event, implicit);
                    } else if strncmp(line.as_ptr(), cstr!("+MAP"), 4) == 0 {
                        style = YAML_BLOCK_MAPPING_STYLE as c_int;
                        if flow == 1 {
                            style = YAML_FLOW_MAPPING_STYLE as c_int;
                        } else if flow == 0
                            && strncmp(line.as_ptr().offset(5), cstr!("{}"), 2) == 0
                        {
                            style = YAML_FLOW_MAPPING_STYLE as c_int;
                        }
                        ok = yaml_mapping_start_event_initialize(
                            &mut event,
                            get_anchor(b'&' as c_char, line.as_mut_ptr(), anchor.as_mut_ptr())
                                as *mut yaml_char_t,
                            get_tag(line.as_mut_ptr(), tag.as_mut_ptr()) as *mut yaml_char_t,
                            0,
                            style as yaml_mapping_style_t,
                        );
                    } else if strncmp(line.as_ptr(), cstr!("-MAP"), 4) == 0 {
                        ok = yaml_mapping_end_event_initialize(&mut event);
                    } else if strncmp(line.as_ptr(), cstr!("+SEQ"), 4) == 0 {
                        style = YAML_BLOCK_SEQUENCE_STYLE as c_int;
                        if flow == 1 {
                            style = YAML_FLOW_MAPPING_STYLE as c_int;
                        } else if flow == 0
                            && strncmp(line.as_ptr().offset(5), cstr!("[]"), 2) == 0
                        {
                            style = YAML_FLOW_SEQUENCE_STYLE as c_int;
                        }
                        ok = yaml_sequence_start_event_initialize(
                            &mut event,
                            get_anchor(b'&' as c_char, line.as_mut_ptr(), anchor.as_mut_ptr())
                                as *mut yaml_char_t,
                            get_tag(line.as_mut_ptr(), tag.as_mut_ptr()) as *mut yaml_char_t,
                            0,
                            style as yaml_sequence_style_t,
                        );
                    } else if strncmp(line.as_ptr(), cstr!("-SEQ"), 4) == 0 {
                        ok = yaml_sequence_end_event_initialize(&mut event);
                    } else if strncmp(line.as_ptr(), cstr!("=VAL"), 4) == 0 {
                        let mut value: [c_char; 1024] = [0; 1024];
                        let mut style: c_int = 0;

                        get_value(line.as_mut_ptr(), value.as_mut_ptr(), &mut style);
                        implicit = (get_tag(line.as_mut_ptr(), tag.as_mut_ptr()).is_null()) as c_int;

                        ok = yaml_scalar_event_initialize(
                            &mut event,
                            get_anchor(b'&' as c_char, line.as_mut_ptr(), anchor.as_mut_ptr())
                                as *mut yaml_char_t,
                            get_tag(line.as_mut_ptr(), tag.as_mut_ptr()) as *mut yaml_char_t,
                            value.as_mut_ptr() as *mut yaml_char_t,
                            -1,
                            implicit,
                            implicit,
                            style as yaml_scalar_style_t,
                        );
                    } else if strncmp(line.as_ptr(), cstr!("=ALI"), 4) == 0 {
                        ok = yaml_alias_event_initialize(
                            &mut event,
                            get_anchor(b'*' as c_char, line.as_mut_ptr(), anchor.as_mut_ptr())
                                as *mut yaml_char_t,
                        );
                    } else {
                        fprintf(stderr, cstr!("Unknown event: '%s'\n"), line.as_ptr());
                        fflush(stdout);
                        return 1;
                    }

                    if ok == 0 {
                        break 'event_error;
                    }
                    if yaml_emitter_emit(&mut emitter, &mut event) == 0 {
                        break 'emitter_error;
                    }
                }

                c_assert!(fclose(input) == 0);
                yaml_emitter_delete(&mut emitter);
                fflush(stdout);

                return 0;
            }

            /* event_error: */
            fprintf(
                stderr,
                cstr!("Memory error: Not enough memory for creating an event\n"),
            );
            yaml_emitter_delete(&mut emitter);
            return 1;
        }

        /* emitter_error: */
        match emitter.error {
            YAML_MEMORY_ERROR => {
                fprintf(
                    stderr,
                    cstr!("Memory error: Not enough memory for emitting\n"),
                );
            }
            YAML_WRITER_ERROR => {
                fprintf(stderr, cstr!("Writer error: %s\n"), emitter.problem);
            }
            YAML_EMITTER_ERROR => {
                fprintf(stderr, cstr!("Emitter error: %s\n"), emitter.problem);
            }
            _ => {
                /*
                 * Couldn't happen.
                 */
                fprintf(stderr, cstr!("Internal error\n"));
            }
        }
        yaml_emitter_delete(&mut emitter);
        return 1;
    }
}

unsafe fn get_line(input: *mut FILE, line: *mut c_char) -> c_int {
    unsafe {
        let newline: *mut c_char;

        if fgets(line, 1024 - 1, input).is_null() {
            return 0;
        }

        newline = strchr(line, b'\n' as c_int);
        if newline.is_null() {
            fprintf(stderr, cstr!("Line too long: '%s'"), line as *const c_char);
            abort();
        }
        *newline = b'\0' as c_char;

        return 1;
    }
}

unsafe fn get_anchor(sigil: c_char, line: *mut c_char, anchor: *mut c_char) -> *mut c_char {
    unsafe {
        let mut start: *mut c_char;
        let mut end: *mut c_char;

        start = strchr(line, sigil as c_int);
        if start.is_null() {
            return core::ptr::null_mut();
        }
        start = start.offset(1);
        end = strchr(start, b' ' as c_int);
        if end.is_null() {
            end = line.offset(strlen(line) as isize);
        }
        memcpy(
            anchor as *mut c_void,
            start as *const c_void,
            end.offset_from(start) as size_t,
        );
        *anchor.offset(end.offset_from(start)) = b'\0' as c_char;
        return anchor;
    }
}

unsafe fn get_tag(line: *mut c_char, tag: *mut c_char) -> *mut c_char {
    unsafe {
        let start: *mut c_char;
        let end: *mut c_char;

        start = strchr(line, b'<' as c_int);
        if start.is_null() {
            return core::ptr::null_mut();
        }
        end = strchr(line, b'>' as c_int);
        if end.is_null() {
            return core::ptr::null_mut();
        }
        memcpy(
            tag as *mut c_void,
            start.offset(1) as *const c_void,
            (end.offset_from(start) - 1) as size_t,
        );
        *tag.offset(end.offset_from(start) - 1) = b'\0' as c_char;
        return tag;
    }
}

unsafe fn get_value(line: *mut c_char, value: *mut c_char, style: *mut c_int) {
    unsafe {
        let mut i: c_int = 0;
        let mut c: *mut c_char;
        let mut start: *mut c_char = core::ptr::null_mut();
        let end: *mut c_char = line.offset(strlen(line) as isize);

        c = line.offset(4);
        while c < end {
            if *c == b' ' as c_char {
                start = c.offset(1);
                if *start == b':' as c_char {
                    *style = YAML_PLAIN_SCALAR_STYLE as c_int;
                } else if *start == b'\'' as c_char {
                    *style = YAML_SINGLE_QUOTED_SCALAR_STYLE as c_int;
                } else if *start == b'"' as c_char {
                    *style = YAML_DOUBLE_QUOTED_SCALAR_STYLE as c_int;
                } else if *start == b'|' as c_char {
                    *style = YAML_LITERAL_SCALAR_STYLE as c_int;
                } else if *start == b'>' as c_char {
                    *style = YAML_FOLDED_SCALAR_STYLE as c_int;
                } else {
                    start = core::ptr::null_mut();
                    c = c.offset(1);
                    continue;
                }
                start = start.offset(1);
                break;
            }
            c = c.offset(1);
        }
        if start.is_null() {
            abort();
        }

        c = start;
        while c < end {
            if *c == b'\\' as c_char {
                c = c.offset(1);
                if *c == b'\\' as c_char {
                    *value.offset(i as isize) = b'\\' as c_char;
                    i += 1;
                } else if *c == b'0' as c_char {
                    *value.offset(i as isize) = b'\0' as c_char;
                    i += 1;
                } else if *c == b'b' as c_char {
                    *value.offset(i as isize) = 0x08 as c_char;
                    i += 1;
                } else if *c == b'n' as c_char {
                    *value.offset(i as isize) = b'\n' as c_char;
                    i += 1;
                } else if *c == b'r' as c_char {
                    *value.offset(i as isize) = b'\r' as c_char;
                    i += 1;
                } else if *c == b't' as c_char {
                    *value.offset(i as isize) = b'\t' as c_char;
                    i += 1;
                } else {
                    abort();
                }
            } else {
                *value.offset(i as isize) = *c;
                i += 1;
            }
            c = c.offset(1);
        }
        *value.offset(i as isize) = b'\0' as c_char;
    }
}

unsafe fn usage(ret: c_int) -> c_int {
    unsafe {
        fprintf(
            stderr,
            cstr!("Usage: run-emitter-test-suite [--directive (1.1|1.2)] [--flow (on|off|keep)] [<input-file>]\n"),
        );
        return ret;
    }
}

fn main() {
    ctest::run(c_main)
}
