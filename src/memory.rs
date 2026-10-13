//! Translation of the memory-management, string, stack and queue helpers at the
//! top of `c_src/src/api.c` (version info included).
//!
//! These live in their own module because `src/private.rs` depends on them.
//! The exported linker symbols are unchanged.

use crate::libc_shim::{
    c_char, c_int, c_void, free, malloc, memcpy, memset, realloc, size_t, strdup, INT_MAX,
};
use crate::yaml_char_t;
use crate::{YAML_VERSION_MAJOR, YAML_VERSION_MINOR, YAML_VERSION_PATCH, YAML_VERSION_STRING};

/*
 * Get the library version.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_get_version_string() -> *const c_char {
    YAML_VERSION_STRING.as_ptr() as *const c_char
}

/*
 * Get the library version numbers.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_get_version(major: *mut c_int, minor: *mut c_int, patch: *mut c_int) {
    *major = YAML_VERSION_MAJOR;
    *minor = YAML_VERSION_MINOR;
    *patch = YAML_VERSION_PATCH;
}

/*
 * Allocate a dynamic memory block.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_malloc(size: size_t) -> *mut c_void {
    malloc(if size != 0 { size } else { 1 })
}

/*
 * Reallocate a dynamic memory block.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_realloc(ptr: *mut c_void, size: size_t) -> *mut c_void {
    if !ptr.is_null() {
        realloc(ptr, if size != 0 { size } else { 1 })
    } else {
        malloc(if size != 0 { size } else { 1 })
    }
}

/*
 * Free a dynamic memory block.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        free(ptr);
    }
}

/*
 * Duplicate a string.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_strdup(str_: *const yaml_char_t) -> *mut yaml_char_t {
    if str_.is_null() {
        return core::ptr::null_mut();
    }

    strdup(str_ as *const c_char) as *mut yaml_char_t
}

/*
 * Extend a string.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_string_extend(
    start: *mut *mut yaml_char_t,
    pointer: *mut *mut yaml_char_t,
    end: *mut *mut yaml_char_t,
) -> c_int {
    let old_len = (*end).offset_from(*start) as size_t;
    let new_start = yaml_realloc(*start as *mut c_void, old_len * 2) as *mut yaml_char_t;

    if new_start.is_null() {
        return 0;
    }

    memset(new_start.add(old_len) as *mut c_void, 0, old_len);

    *pointer = new_start.offset((*pointer).offset_from(*start));
    *end = new_start.add(old_len * 2);
    *start = new_start;

    1
}

/*
 * Append a string B to a string A.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_string_join(
    a_start: *mut *mut yaml_char_t,
    a_pointer: *mut *mut yaml_char_t,
    a_end: *mut *mut yaml_char_t,
    b_start: *mut *mut yaml_char_t,
    b_pointer: *mut *mut yaml_char_t,
    b_end: *mut *mut yaml_char_t,
) -> c_int {
    let _ = b_end;

    if *b_start == *b_pointer {
        return 1;
    }

    while (*a_end).offset_from(*a_pointer) <= (*b_pointer).offset_from(*b_start) {
        if yaml_string_extend(a_start, a_pointer, a_end) == 0 {
            return 0;
        }
    }

    memcpy(
        *a_pointer as *mut c_void,
        *b_start as *const c_void,
        (*b_pointer).offset_from(*b_start) as size_t,
    );
    *a_pointer = (*a_pointer).offset((*b_pointer).offset_from(*b_start));

    1
}

/*
 * Extend a stack.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_stack_extend(
    start: *mut *mut c_void,
    top: *mut *mut c_void,
    end: *mut *mut c_void,
) -> c_int {
    let size = (*end as *mut c_char).offset_from(*start as *mut c_char);

    if size >= (INT_MAX / 2) as isize {
        return 0;
    }

    let new_start = yaml_realloc(*start, (size * 2) as size_t);

    if new_start.is_null() {
        return 0;
    }

    *top = (new_start as *mut c_char)
        .offset((*top as *mut c_char).offset_from(*start as *mut c_char))
        as *mut c_void;
    *end = (new_start as *mut c_char).offset(size * 2) as *mut c_void;
    *start = new_start;

    1
}

/*
 * Extend or move a queue.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yaml_queue_extend(
    start: *mut *mut c_void,
    head: *mut *mut c_void,
    tail: *mut *mut c_void,
    end: *mut *mut c_void,
) -> c_int {
    /* Check if we need to resize the queue. */

    if *start == *head && *tail == *end {
        let size = (*end as *mut c_char).offset_from(*start as *mut c_char);
        let new_start = yaml_realloc(*start, (size * 2) as size_t);

        if new_start.is_null() {
            return 0;
        }

        *head = (new_start as *mut c_char)
            .offset((*head as *mut c_char).offset_from(*start as *mut c_char))
            as *mut c_void;
        *tail = (new_start as *mut c_char)
            .offset((*tail as *mut c_char).offset_from(*start as *mut c_char))
            as *mut c_void;
        *end = (new_start as *mut c_char).offset(size * 2) as *mut c_void;
        *start = new_start;
    }

    /* Check if we need to move the queue at the beginning of the buffer. */

    if *tail == *end {
        if *head != *tail {
            crate::libc_shim::memmove(
                *start,
                *head,
                (*tail as *mut c_char).offset_from(*head as *mut c_char) as size_t,
            );
        }
        *tail = (*start as *mut c_char)
            .offset((*tail as *mut c_char).offset_from(*head as *mut c_char))
            as *mut c_void;
        *head = *start;
    }

    1
}
