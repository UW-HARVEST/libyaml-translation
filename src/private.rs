//! Translation of `c_src/src/yaml_private.h`.
//!
//! The C preprocessor macros become `macro_rules!` macros with the same names
//! and the same argument order.  They are "duck typed" on the field names
//! (`.pointer`, `.start`, `.end`, `.top`, `.head`, `.tail`) exactly like the C
//! originals, so they work on `yaml_string_t`, `yaml_buffer_t`,
//! `yaml_stack_t<T>` and `yaml_queue_t<T>` alike.
//!
//! Differences from C that every caller must know:
//!
//! * Macros that C returns `int` (1/0) from return **`bool`** here.  Use
//!   `if !FOO!(..) { .. }` instead of `if (!FOO(..))`.  Where an `int` is
//!   genuinely needed, write `FOO!(..) as c_int`.
//! * `STACK_INIT`/`QUEUE_INIT` drop the trailing C `type` argument (the element
//!   type is inferred).
//! * `POP!`/`DEQUEUE!` return the value; `yaml_token_delete(&DEQUEUE(..))`
//!   becomes
//!   `let mut t = DEQUEUE!(ctx, q); yaml_token_delete(&mut t);`.
//! * All macros expand to `unsafe` operations and must be used inside an
//!   `unsafe` context.

use crate::libc_shim::{c_char, c_int, c_uint, c_void, memmove, memset, size_t};
use crate::yaml_char_t;
use crate::{yaml_queue_t, yaml_stack_t};

/* ------------------------------------------------------------------ */
/* Sizes                                                               */
/* ------------------------------------------------------------------ */

/// The size of the input raw buffer.
pub const INPUT_RAW_BUFFER_SIZE: size_t = 16384;

/// The size of the input buffer.
///
/// It should be possible to decode the whole raw buffer.
pub const INPUT_BUFFER_SIZE: size_t = INPUT_RAW_BUFFER_SIZE * 3;

/// The size of the output buffer.
pub const OUTPUT_BUFFER_SIZE: size_t = 16384;

/// The size of the output raw buffer.
///
/// It should be possible to encode the whole output buffer.
pub const OUTPUT_RAW_BUFFER_SIZE: size_t = OUTPUT_BUFFER_SIZE * 2 + 2;

/// The maximum size of a YAML input file.
pub const MAX_FILE_SIZE: size_t = (!(0 as size_t)) / 2;

/// The size of other stacks and queues.
pub const INITIAL_STACK_SIZE: size_t = 16;
pub const INITIAL_QUEUE_SIZE: size_t = 16;
pub const INITIAL_STRING_SIZE: size_t = 16;

/* ------------------------------------------------------------------ */
/* String management                                                   */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_string_t {
    pub start: *mut yaml_char_t,
    pub end: *mut yaml_char_t,
    pub pointer: *mut yaml_char_t,
}

impl yaml_string_t {
    #[inline]
    pub const fn null() -> Self {
        yaml_string_t {
            start: core::ptr::null_mut(),
            end: core::ptr::null_mut(),
            pointer: core::ptr::null_mut(),
        }
    }
}

/* ------------------------------------------------------------------ */
/* Generic stack / queue helpers (element type inferred)               */
/* ------------------------------------------------------------------ */

#[inline]
pub unsafe fn stack_extend_raw<T>(stack: *mut yaml_stack_t<T>) -> bool {
    crate::memory::yaml_stack_extend(
        (&raw mut (*stack).start) as *mut *mut c_void,
        (&raw mut (*stack).top) as *mut *mut c_void,
        (&raw mut (*stack).end) as *mut *mut c_void,
    ) != 0
}

#[inline]
pub unsafe fn queue_extend_raw<T>(queue: *mut yaml_queue_t<T>) -> bool {
    crate::memory::yaml_queue_extend(
        (&raw mut (*queue).start) as *mut *mut c_void,
        (&raw mut (*queue).head) as *mut *mut c_void,
        (&raw mut (*queue).tail) as *mut *mut c_void,
        (&raw mut (*queue).end) as *mut *mut c_void,
    ) != 0
}

/// `STACK_INIT` body: allocate `INITIAL_STACK_SIZE` elements.
#[inline]
pub unsafe fn stack_init_raw<T>(stack: *mut yaml_stack_t<T>) -> bool {
    let bytes = INITIAL_STACK_SIZE * core::mem::size_of::<T>();
    let start = crate::memory::yaml_malloc(bytes) as *mut T;
    (*stack).start = start;
    if !start.is_null() {
        (*stack).top = start;
        (*stack).end = start.add(INITIAL_STACK_SIZE);
        true
    } else {
        false
    }
}

/// `QUEUE_INIT` body: allocate `size` elements.
#[inline]
pub unsafe fn queue_init_raw<T>(queue: *mut yaml_queue_t<T>, size: size_t) -> bool {
    let bytes = size * core::mem::size_of::<T>();
    let start = crate::memory::yaml_malloc(bytes) as *mut T;
    (*queue).start = start;
    if !start.is_null() {
        (*queue).head = start;
        (*queue).tail = start;
        (*queue).end = start.add(size);
        true
    } else {
        false
    }
}

/// `QUEUE_INSERT` body (without the error assignment).
#[inline]
pub unsafe fn queue_insert_raw<T: Copy>(
    queue: *mut yaml_queue_t<T>,
    index: size_t,
    value: T,
) -> bool {
    if (*queue).tail == (*queue).end && !queue_extend_raw(queue) {
        return false;
    }
    let idx = index as isize;
    let count = (*queue).tail.offset_from((*queue).head) - idx;
    memmove(
        (*queue).head.offset(idx + 1) as *mut c_void,
        (*queue).head.offset(idx) as *const c_void,
        (count as size_t) * core::mem::size_of::<T>(),
    );
    *(*queue).head.offset(idx) = value;
    (*queue).tail = (*queue).tail.offset(1);
    true
}

/* ------------------------------------------------------------------ */
/* Buffer management                                                   */
/* ------------------------------------------------------------------ */

#[macro_export]
macro_rules! BUFFER_INIT {
    ($context:expr, $buffer:expr, $size:expr) => {{
        let __size: $crate::libc_shim::size_t = ($size) as $crate::libc_shim::size_t;
        $buffer.start = $crate::memory::yaml_malloc(__size) as *mut $crate::yaml_char_t;
        if !$buffer.start.is_null() {
            $buffer.last = $buffer.start;
            $buffer.pointer = $buffer.start;
            $buffer.end = $buffer.start.add(__size);
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

#[macro_export]
macro_rules! BUFFER_DEL {
    ($context:expr, $buffer:expr) => {{
        $crate::memory::yaml_free($buffer.start as *mut $crate::libc_shim::c_void);
        $buffer.start = core::ptr::null_mut();
        $buffer.pointer = core::ptr::null_mut();
        $buffer.end = core::ptr::null_mut();
    }};
}

/* ------------------------------------------------------------------ */
/* String macros                                                       */
/* ------------------------------------------------------------------ */

#[macro_export]
macro_rules! NULL_STRING {
    () => {
        $crate::private::yaml_string_t::null()
    };
}

/// `STRING(string,length)` -> a `yaml_string_t` value.
#[macro_export]
macro_rules! STRING {
    ($string:expr, $length:expr) => {{
        let __s: *mut $crate::yaml_char_t = $string;
        $crate::private::yaml_string_t {
            start: __s,
            end: __s.add(($length) as usize),
            pointer: __s,
        }
    }};
}

#[macro_export]
macro_rules! STRING_ASSIGN {
    ($value:expr, $string:expr, $length:expr) => {{
        let __s: *mut $crate::yaml_char_t = $string;
        $value.start = __s;
        $value.end = __s.add(($length) as usize);
        $value.pointer = __s;
    }};
}

#[macro_export]
macro_rules! STRING_INIT {
    ($context:expr, $string:expr, $size:expr) => {{
        let __size: $crate::libc_shim::size_t = ($size) as $crate::libc_shim::size_t;
        $string.start = $crate::memory::yaml_malloc(__size) as *mut $crate::yaml_char_t;
        if !$string.start.is_null() {
            $string.pointer = $string.start;
            $string.end = $string.start.add(__size);
            $crate::libc_shim::memset(
                $string.start as *mut $crate::libc_shim::c_void,
                0,
                __size,
            );
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

#[macro_export]
macro_rules! STRING_DEL {
    ($context:expr, $string:expr) => {{
        $crate::memory::yaml_free($string.start as *mut $crate::libc_shim::c_void);
        $string.start = core::ptr::null_mut();
        $string.pointer = core::ptr::null_mut();
        $string.end = core::ptr::null_mut();
    }};
}

#[macro_export]
macro_rules! STRING_EXTEND {
    ($context:expr, $string:expr) => {{
        if $string.pointer.add(5) < $string.end
            || $crate::memory::yaml_string_extend(
                &raw mut $string.start,
                &raw mut $string.pointer,
                &raw mut $string.end,
            ) != 0
        {
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

#[macro_export]
macro_rules! CLEAR {
    ($context:expr, $string:expr) => {{
        $string.pointer = $string.start;
        $crate::libc_shim::memset(
            $string.start as *mut $crate::libc_shim::c_void,
            0,
            $string.end.offset_from($string.start) as $crate::libc_shim::size_t,
        );
    }};
}

#[macro_export]
macro_rules! JOIN {
    ($context:expr, $string_a:expr, $string_b:expr) => {{
        if $crate::memory::yaml_string_join(
            &raw mut $string_a.start,
            &raw mut $string_a.pointer,
            &raw mut $string_a.end,
            &raw mut $string_b.start,
            &raw mut $string_b.pointer,
            &raw mut $string_b.end,
        ) != 0
        {
            $string_b.pointer = $string_b.start;
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

/* ------------------------------------------------------------------ */
/* String check operations                                             */
/* ------------------------------------------------------------------ */

#[macro_export]
macro_rules! CHECK_AT {
    ($string:expr, $octet:expr, $offset:expr) => {
        (*$string.pointer.add(($offset) as usize) == ($octet) as $crate::yaml_char_t)
    };
}

#[macro_export]
macro_rules! CHECK {
    ($string:expr, $octet:expr) => {
        $crate::CHECK_AT!($string, $octet, 0)
    };
}

#[macro_export]
macro_rules! AT {
    ($string:expr, $offset:expr) => {
        (*$string.pointer.add(($offset) as usize))
    };
}

#[macro_export]
macro_rules! IS_ALPHA_AT {
    ($string:expr, $offset:expr) => {{
        let __c = $crate::AT!($string, $offset);
        (__c >= b'0' && __c <= b'9')
            || (__c >= b'A' && __c <= b'Z')
            || (__c >= b'a' && __c <= b'z')
            || __c == b'_'
            || __c == b'-'
    }};
}

#[macro_export]
macro_rules! IS_ALPHA {
    ($string:expr) => {
        $crate::IS_ALPHA_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_DIGIT_AT {
    ($string:expr, $offset:expr) => {{
        let __c = $crate::AT!($string, $offset);
        __c >= b'0' && __c <= b'9'
    }};
}

#[macro_export]
macro_rules! IS_DIGIT {
    ($string:expr) => {
        $crate::IS_DIGIT_AT!($string, 0)
    };
}

/// Value of a digit, as C `int`.
#[macro_export]
macro_rules! AS_DIGIT_AT {
    ($string:expr, $offset:expr) => {
        ($crate::AT!($string, $offset) as $crate::libc_shim::c_int - b'0' as $crate::libc_shim::c_int)
    };
}

#[macro_export]
macro_rules! AS_DIGIT {
    ($string:expr) => {
        $crate::AS_DIGIT_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_HEX_AT {
    ($string:expr, $offset:expr) => {{
        let __c = $crate::AT!($string, $offset);
        (__c >= b'0' && __c <= b'9')
            || (__c >= b'A' && __c <= b'F')
            || (__c >= b'a' && __c <= b'f')
    }};
}

#[macro_export]
macro_rules! IS_HEX {
    ($string:expr) => {
        $crate::IS_HEX_AT!($string, 0)
    };
}

/// Value of a hex digit, as C `int`.
#[macro_export]
macro_rules! AS_HEX_AT {
    ($string:expr, $offset:expr) => {{
        let __c = $crate::AT!($string, $offset) as $crate::libc_shim::c_int;
        if __c >= b'A' as $crate::libc_shim::c_int && __c <= b'F' as $crate::libc_shim::c_int {
            __c - b'A' as $crate::libc_shim::c_int + 10
        } else if __c >= b'a' as $crate::libc_shim::c_int && __c <= b'f' as $crate::libc_shim::c_int
        {
            __c - b'a' as $crate::libc_shim::c_int + 10
        } else {
            __c - b'0' as $crate::libc_shim::c_int
        }
    }};
}

#[macro_export]
macro_rules! AS_HEX {
    ($string:expr) => {
        $crate::AS_HEX_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_ASCII_AT {
    ($string:expr, $offset:expr) => {
        ($crate::AT!($string, $offset) <= 0x7F)
    };
}

#[macro_export]
macro_rules! IS_ASCII {
    ($string:expr) => {
        $crate::IS_ASCII_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_PRINTABLE_AT {
    ($string:expr, $offset:expr) => {{
        let __o = ($offset) as usize;
        let __p = $string.pointer;
        let __c = *__p.add(__o);
        (__c == 0x0A)
            || (__c >= 0x20 && __c <= 0x7E)
            || (__c == 0xC2 && *__p.add(__o + 1) >= 0xA0)
            || (__c > 0xC2 && __c < 0xED)
            || (__c == 0xED && *__p.add(__o + 1) < 0xA0)
            || (__c == 0xEE)
            || (__c == 0xEF
                && !(*__p.add(__o + 1) == 0xBB && *__p.add(__o + 2) == 0xBF)
                && !(*__p.add(__o + 1) == 0xBF
                    && (*__p.add(__o + 2) == 0xBE || *__p.add(__o + 2) == 0xBF)))
    }};
}

#[macro_export]
macro_rules! IS_PRINTABLE {
    ($string:expr) => {
        $crate::IS_PRINTABLE_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_Z_AT {
    ($string:expr, $offset:expr) => {
        $crate::CHECK_AT!($string, b'\0', $offset)
    };
}

#[macro_export]
macro_rules! IS_Z {
    ($string:expr) => {
        $crate::IS_Z_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_BOM_AT {
    ($string:expr, $offset:expr) => {{
        let __o = ($offset) as usize;
        $crate::CHECK_AT!($string, 0xEFu8, __o)
            && $crate::CHECK_AT!($string, 0xBBu8, __o + 1)
            && $crate::CHECK_AT!($string, 0xBFu8, __o + 2)
    }};
}

#[macro_export]
macro_rules! IS_BOM {
    ($string:expr) => {
        $crate::IS_BOM_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_SPACE_AT {
    ($string:expr, $offset:expr) => {
        $crate::CHECK_AT!($string, b' ', $offset)
    };
}

#[macro_export]
macro_rules! IS_SPACE {
    ($string:expr) => {
        $crate::IS_SPACE_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_TAB_AT {
    ($string:expr, $offset:expr) => {
        $crate::CHECK_AT!($string, b'\t', $offset)
    };
}

#[macro_export]
macro_rules! IS_TAB {
    ($string:expr) => {
        $crate::IS_TAB_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_BLANK_AT {
    ($string:expr, $offset:expr) => {{
        let __o = ($offset) as usize;
        $crate::IS_SPACE_AT!($string, __o) || $crate::IS_TAB_AT!($string, __o)
    }};
}

#[macro_export]
macro_rules! IS_BLANK {
    ($string:expr) => {
        $crate::IS_BLANK_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_BREAK_AT {
    ($string:expr, $offset:expr) => {{
        let __o = ($offset) as usize;
        $crate::CHECK_AT!($string, b'\r', __o)
            || $crate::CHECK_AT!($string, b'\n', __o)
            || ($crate::CHECK_AT!($string, 0xC2u8, __o)
                && $crate::CHECK_AT!($string, 0x85u8, __o + 1))
            || ($crate::CHECK_AT!($string, 0xE2u8, __o)
                && $crate::CHECK_AT!($string, 0x80u8, __o + 1)
                && $crate::CHECK_AT!($string, 0xA8u8, __o + 2))
            || ($crate::CHECK_AT!($string, 0xE2u8, __o)
                && $crate::CHECK_AT!($string, 0x80u8, __o + 1)
                && $crate::CHECK_AT!($string, 0xA9u8, __o + 2))
    }};
}

#[macro_export]
macro_rules! IS_BREAK {
    ($string:expr) => {
        $crate::IS_BREAK_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_CRLF_AT {
    ($string:expr, $offset:expr) => {{
        let __o = ($offset) as usize;
        $crate::CHECK_AT!($string, b'\r', __o) && $crate::CHECK_AT!($string, b'\n', __o + 1)
    }};
}

#[macro_export]
macro_rules! IS_CRLF {
    ($string:expr) => {
        $crate::IS_CRLF_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_BREAKZ_AT {
    ($string:expr, $offset:expr) => {{
        let __o = ($offset) as usize;
        $crate::IS_BREAK_AT!($string, __o) || $crate::IS_Z_AT!($string, __o)
    }};
}

#[macro_export]
macro_rules! IS_BREAKZ {
    ($string:expr) => {
        $crate::IS_BREAKZ_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_SPACEZ_AT {
    ($string:expr, $offset:expr) => {{
        let __o = ($offset) as usize;
        $crate::IS_SPACE_AT!($string, __o) || $crate::IS_BREAKZ_AT!($string, __o)
    }};
}

#[macro_export]
macro_rules! IS_SPACEZ {
    ($string:expr) => {
        $crate::IS_SPACEZ_AT!($string, 0)
    };
}

#[macro_export]
macro_rules! IS_BLANKZ_AT {
    ($string:expr, $offset:expr) => {{
        let __o = ($offset) as usize;
        $crate::IS_BLANK_AT!($string, __o) || $crate::IS_BREAKZ_AT!($string, __o)
    }};
}

#[macro_export]
macro_rules! IS_BLANKZ {
    ($string:expr) => {
        $crate::IS_BLANKZ_AT!($string, 0)
    };
}

/// Width of the character, as C `int` (0 for an invalid lead octet).
#[macro_export]
macro_rules! WIDTH_AT {
    ($string:expr, $offset:expr) => {{
        let __c = $crate::AT!($string, $offset);
        if (__c & 0x80) == 0x00 {
            1 as $crate::libc_shim::c_int
        } else if (__c & 0xE0) == 0xC0 {
            2 as $crate::libc_shim::c_int
        } else if (__c & 0xF0) == 0xE0 {
            3 as $crate::libc_shim::c_int
        } else if (__c & 0xF8) == 0xF0 {
            4 as $crate::libc_shim::c_int
        } else {
            0 as $crate::libc_shim::c_int
        }
    }};
}

#[macro_export]
macro_rules! WIDTH {
    ($string:expr) => {
        $crate::WIDTH_AT!($string, 0)
    };
}

/// Move the string pointer to the next character.
#[macro_export]
macro_rules! MOVE {
    ($string:expr) => {{
        $string.pointer = $string.pointer.add($crate::WIDTH!($string) as usize);
    }};
}

/// Copy a character and move the pointers of both strings.
#[macro_export]
macro_rules! COPY {
    ($string_a:expr, $string_b:expr) => {{
        let __w = {
            let __c = *$string_b.pointer;
            if (__c & 0x80) == 0x00 {
                1usize
            } else if (__c & 0xE0) == 0xC0 {
                2usize
            } else if (__c & 0xF0) == 0xE0 {
                3usize
            } else if (__c & 0xF8) == 0xF0 {
                4usize
            } else {
                0usize
            }
        };
        let mut __i = 0usize;
        while __i < __w {
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.add(1);
            $string_b.pointer = $string_b.pointer.add(1);
            __i += 1;
        }
    }};
}

/* ------------------------------------------------------------------ */
/* Stack and queue management                                          */
/* ------------------------------------------------------------------ */

/// `STACK_INIT(context, stack, type)` — the C `type` argument is dropped.
#[macro_export]
macro_rules! STACK_INIT {
    ($context:expr, $stack:expr) => {{
        if $crate::private::stack_init_raw(&raw mut $stack) {
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

#[macro_export]
macro_rules! STACK_DEL {
    ($context:expr, $stack:expr) => {{
        $crate::memory::yaml_free($stack.start as *mut $crate::libc_shim::c_void);
        $stack.start = core::ptr::null_mut();
        $stack.top = core::ptr::null_mut();
        $stack.end = core::ptr::null_mut();
    }};
}

#[macro_export]
macro_rules! STACK_EMPTY {
    ($context:expr, $stack:expr) => {
        ($stack.start == $stack.top)
    };
}

#[macro_export]
macro_rules! STACK_LIMIT {
    ($context:expr, $stack:expr, $size:expr) => {{
        if $stack.top.offset_from($stack.start) < ($size) as isize {
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

#[macro_export]
macro_rules! PUSH {
    ($context:expr, $stack:expr, $value:expr) => {{
        if $stack.top != $stack.end || $crate::private::stack_extend_raw(&raw mut $stack) {
            *$stack.top = $value;
            $stack.top = $stack.top.add(1);
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

#[macro_export]
macro_rules! POP {
    ($context:expr, $stack:expr) => {{
        $stack.top = $stack.top.sub(1);
        *$stack.top
    }};
}

/// `QUEUE_INIT(context, queue, size, type)` — the C `type` argument is dropped.
#[macro_export]
macro_rules! QUEUE_INIT {
    ($context:expr, $queue:expr, $size:expr) => {{
        if $crate::private::queue_init_raw(
            &raw mut $queue,
            ($size) as $crate::libc_shim::size_t,
        ) {
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

#[macro_export]
macro_rules! QUEUE_DEL {
    ($context:expr, $queue:expr) => {{
        $crate::memory::yaml_free($queue.start as *mut $crate::libc_shim::c_void);
        $queue.start = core::ptr::null_mut();
        $queue.head = core::ptr::null_mut();
        $queue.tail = core::ptr::null_mut();
        $queue.end = core::ptr::null_mut();
    }};
}

#[macro_export]
macro_rules! QUEUE_EMPTY {
    ($context:expr, $queue:expr) => {
        ($queue.head == $queue.tail)
    };
}

#[macro_export]
macro_rules! ENQUEUE {
    ($context:expr, $queue:expr, $value:expr) => {{
        if $queue.tail != $queue.end || $crate::private::queue_extend_raw(&raw mut $queue) {
            *$queue.tail = $value;
            $queue.tail = $queue.tail.add(1);
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

#[macro_export]
macro_rules! DEQUEUE {
    ($context:expr, $queue:expr) => {{
        let __v = *$queue.head;
        $queue.head = $queue.head.add(1);
        __v
    }};
}

#[macro_export]
macro_rules! QUEUE_INSERT {
    ($context:expr, $queue:expr, $index:expr, $value:expr) => {{
        if $crate::private::queue_insert_raw(
            &raw mut $queue,
            ($index) as $crate::libc_shim::size_t,
            $value,
        ) {
            true
        } else {
            (*$context).error = $crate::YAML_MEMORY_ERROR;
            false
        }
    }};
}

/* ------------------------------------------------------------------ */
/* Token initializers                                                  */
/* ------------------------------------------------------------------ */

#[macro_export]
macro_rules! TOKEN_INIT {
    ($token:expr, $token_type:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::libc_shim::memset(
            (&raw mut $token) as *mut $crate::libc_shim::c_void,
            0,
            core::mem::size_of::<$crate::yaml_token_t>(),
        );
        $token.type_ = $token_type;
        $token.start_mark = $start_mark;
        $token.end_mark = $end_mark;
    }};
}

#[macro_export]
macro_rules! STREAM_START_TOKEN_INIT {
    ($token:expr, $encoding:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::TOKEN_INIT!(
            $token,
            $crate::YAML_STREAM_START_TOKEN,
            $start_mark,
            $end_mark
        );
        $token.data.stream_start.encoding = $encoding;
    }};
}

#[macro_export]
macro_rules! STREAM_END_TOKEN_INIT {
    ($token:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::TOKEN_INIT!(
            $token,
            $crate::YAML_STREAM_END_TOKEN,
            $start_mark,
            $end_mark
        );
    }};
}

#[macro_export]
macro_rules! ALIAS_TOKEN_INIT {
    ($token:expr, $value:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::TOKEN_INIT!($token, $crate::YAML_ALIAS_TOKEN, $start_mark, $end_mark);
        $token.data.alias.value = $value;
    }};
}

#[macro_export]
macro_rules! ANCHOR_TOKEN_INIT {
    ($token:expr, $value:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::TOKEN_INIT!($token, $crate::YAML_ANCHOR_TOKEN, $start_mark, $end_mark);
        $token.data.anchor.value = $value;
    }};
}

#[macro_export]
macro_rules! TAG_TOKEN_INIT {
    ($token:expr, $handle:expr, $suffix:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::TOKEN_INIT!($token, $crate::YAML_TAG_TOKEN, $start_mark, $end_mark);
        $token.data.tag.handle = $handle;
        $token.data.tag.suffix = $suffix;
    }};
}

#[macro_export]
macro_rules! SCALAR_TOKEN_INIT {
    ($token:expr, $value:expr, $length:expr, $style:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::TOKEN_INIT!($token, $crate::YAML_SCALAR_TOKEN, $start_mark, $end_mark);
        $token.data.scalar.value = $value;
        $token.data.scalar.length = $length;
        $token.data.scalar.style = $style;
    }};
}

#[macro_export]
macro_rules! VERSION_DIRECTIVE_TOKEN_INIT {
    ($token:expr, $major:expr, $minor:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::TOKEN_INIT!(
            $token,
            $crate::YAML_VERSION_DIRECTIVE_TOKEN,
            $start_mark,
            $end_mark
        );
        $token.data.version_directive.major = $major;
        $token.data.version_directive.minor = $minor;
    }};
}

#[macro_export]
macro_rules! TAG_DIRECTIVE_TOKEN_INIT {
    ($token:expr, $handle:expr, $prefix:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::TOKEN_INIT!(
            $token,
            $crate::YAML_TAG_DIRECTIVE_TOKEN,
            $start_mark,
            $end_mark
        );
        $token.data.tag_directive.handle = $handle;
        $token.data.tag_directive.prefix = $prefix;
    }};
}

/* ------------------------------------------------------------------ */
/* Event initializers                                                  */
/* ------------------------------------------------------------------ */

#[macro_export]
macro_rules! EVENT_INIT {
    ($event:expr, $event_type:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::libc_shim::memset(
            (&raw mut $event) as *mut $crate::libc_shim::c_void,
            0,
            core::mem::size_of::<$crate::yaml_event_t>(),
        );
        $event.type_ = $event_type;
        $event.start_mark = $start_mark;
        $event.end_mark = $end_mark;
    }};
}

#[macro_export]
macro_rules! STREAM_START_EVENT_INIT {
    ($event:expr, $encoding:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!(
            $event,
            $crate::YAML_STREAM_START_EVENT,
            $start_mark,
            $end_mark
        );
        $event.data.stream_start.encoding = $encoding;
    }};
}

#[macro_export]
macro_rules! STREAM_END_EVENT_INIT {
    ($event:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!(
            $event,
            $crate::YAML_STREAM_END_EVENT,
            $start_mark,
            $end_mark
        );
    }};
}

#[macro_export]
macro_rules! DOCUMENT_START_EVENT_INIT {
    ($event:expr, $version_directive:expr, $tag_directives_start:expr,
     $tag_directives_end:expr, $implicit:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!(
            $event,
            $crate::YAML_DOCUMENT_START_EVENT,
            $start_mark,
            $end_mark
        );
        $event.data.document_start.version_directive = $version_directive;
        $event.data.document_start.tag_directives.start = $tag_directives_start;
        $event.data.document_start.tag_directives.end = $tag_directives_end;
        $event.data.document_start.implicit = $implicit;
    }};
}

#[macro_export]
macro_rules! DOCUMENT_END_EVENT_INIT {
    ($event:expr, $implicit:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!(
            $event,
            $crate::YAML_DOCUMENT_END_EVENT,
            $start_mark,
            $end_mark
        );
        $event.data.document_end.implicit = $implicit;
    }};
}

#[macro_export]
macro_rules! ALIAS_EVENT_INIT {
    ($event:expr, $anchor:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!($event, $crate::YAML_ALIAS_EVENT, $start_mark, $end_mark);
        $event.data.alias.anchor = $anchor;
    }};
}

#[macro_export]
macro_rules! SCALAR_EVENT_INIT {
    ($event:expr, $anchor:expr, $tag:expr, $value:expr, $length:expr,
     $plain_implicit:expr, $quoted_implicit:expr, $style:expr,
     $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!($event, $crate::YAML_SCALAR_EVENT, $start_mark, $end_mark);
        $event.data.scalar.anchor = $anchor;
        $event.data.scalar.tag = $tag;
        $event.data.scalar.value = $value;
        $event.data.scalar.length = $length;
        $event.data.scalar.plain_implicit = $plain_implicit;
        $event.data.scalar.quoted_implicit = $quoted_implicit;
        $event.data.scalar.style = $style;
    }};
}

#[macro_export]
macro_rules! SEQUENCE_START_EVENT_INIT {
    ($event:expr, $anchor:expr, $tag:expr, $implicit:expr, $style:expr,
     $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!(
            $event,
            $crate::YAML_SEQUENCE_START_EVENT,
            $start_mark,
            $end_mark
        );
        $event.data.sequence_start.anchor = $anchor;
        $event.data.sequence_start.tag = $tag;
        $event.data.sequence_start.implicit = $implicit;
        $event.data.sequence_start.style = $style;
    }};
}

#[macro_export]
macro_rules! SEQUENCE_END_EVENT_INIT {
    ($event:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!(
            $event,
            $crate::YAML_SEQUENCE_END_EVENT,
            $start_mark,
            $end_mark
        );
    }};
}

#[macro_export]
macro_rules! MAPPING_START_EVENT_INIT {
    ($event:expr, $anchor:expr, $tag:expr, $implicit:expr, $style:expr,
     $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!(
            $event,
            $crate::YAML_MAPPING_START_EVENT,
            $start_mark,
            $end_mark
        );
        $event.data.mapping_start.anchor = $anchor;
        $event.data.mapping_start.tag = $tag;
        $event.data.mapping_start.implicit = $implicit;
        $event.data.mapping_start.style = $style;
    }};
}

#[macro_export]
macro_rules! MAPPING_END_EVENT_INIT {
    ($event:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::EVENT_INIT!(
            $event,
            $crate::YAML_MAPPING_END_EVENT,
            $start_mark,
            $end_mark
        );
    }};
}

/* ------------------------------------------------------------------ */
/* Document initializer                                                */
/* ------------------------------------------------------------------ */

#[macro_export]
macro_rules! DOCUMENT_INIT {
    ($document:expr, $nodes_start:expr, $nodes_end:expr, $version_directive:expr,
     $tag_directives_start:expr, $tag_directives_end:expr, $start_implicit:expr,
     $end_implicit:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::libc_shim::memset(
            (&raw mut $document) as *mut $crate::libc_shim::c_void,
            0,
            core::mem::size_of::<$crate::yaml_document_t>(),
        );
        $document.nodes.start = $nodes_start;
        $document.nodes.end = $nodes_end;
        $document.nodes.top = $nodes_start;
        $document.version_directive = $version_directive;
        $document.tag_directives.start = $tag_directives_start;
        $document.tag_directives.end = $tag_directives_end;
        $document.start_implicit = $start_implicit;
        $document.end_implicit = $end_implicit;
        $document.start_mark = $start_mark;
        $document.end_mark = $end_mark;
    }};
}

/* ------------------------------------------------------------------ */
/* Node initializers                                                   */
/* ------------------------------------------------------------------ */

#[macro_export]
macro_rules! NODE_INIT {
    ($node:expr, $node_type:expr, $node_tag:expr, $start_mark:expr, $end_mark:expr) => {{
        $crate::libc_shim::memset(
            (&raw mut $node) as *mut $crate::libc_shim::c_void,
            0,
            core::mem::size_of::<$crate::yaml_node_t>(),
        );
        $node.type_ = $node_type;
        $node.tag = $node_tag;
        $node.start_mark = $start_mark;
        $node.end_mark = $end_mark;
    }};
}

#[macro_export]
macro_rules! SCALAR_NODE_INIT {
    ($node:expr, $node_tag:expr, $value:expr, $length:expr, $style:expr,
     $start_mark:expr, $end_mark:expr) => {{
        $crate::NODE_INIT!(
            $node,
            $crate::YAML_SCALAR_NODE,
            $node_tag,
            $start_mark,
            $end_mark
        );
        $node.data.scalar.value = $value;
        $node.data.scalar.length = $length;
        $node.data.scalar.style = $style;
    }};
}

#[macro_export]
macro_rules! SEQUENCE_NODE_INIT {
    ($node:expr, $node_tag:expr, $items_start:expr, $items_end:expr, $style:expr,
     $start_mark:expr, $end_mark:expr) => {{
        $crate::NODE_INIT!(
            $node,
            $crate::YAML_SEQUENCE_NODE,
            $node_tag,
            $start_mark,
            $end_mark
        );
        $node.data.sequence.items.start = $items_start;
        $node.data.sequence.items.end = $items_end;
        $node.data.sequence.items.top = $items_start;
        $node.data.sequence.style = $style;
    }};
}

#[macro_export]
macro_rules! MAPPING_NODE_INIT {
    ($node:expr, $node_tag:expr, $pairs_start:expr, $pairs_end:expr, $style:expr,
     $start_mark:expr, $end_mark:expr) => {{
        $crate::NODE_INIT!(
            $node,
            $crate::YAML_MAPPING_NODE,
            $node_tag,
            $start_mark,
            $end_mark
        );
        $node.data.mapping.pairs.start = $pairs_start;
        $node.data.mapping.pairs.end = $pairs_end;
        $node.data.mapping.pairs.top = $pairs_start;
        $node.data.mapping.style = $style;
    }};
}

/* ================================================================== */
/* File-local macros from scanner.c / parser.c / emitter.c            */
/*                                                                    */
/* These are `#define`s inside individual .c files in the original,   */
/* but our translation splits those files across several Rust         */
/* modules, so they live here and are shared.                         */
/* ================================================================== */

/// `parser.c`: `int MAX_NESTING_LEVEL = 1000;` (a real exported symbol).
#[unsafe(no_mangle)]
pub static mut MAX_NESTING_LEVEL: c_int = 1000;

/* ---------------- scanner.c ---------------- */

/// Ensure that the buffer contains the required number of characters.
/// Return `true` on success, `false` on failure (reader error or memory error).
#[macro_export]
macro_rules! CACHE {
    ($parser:expr, $length:expr) => {{
        let __len: $crate::libc_shim::size_t = ($length) as $crate::libc_shim::size_t;
        if (*$parser).unread >= __len {
            true
        } else {
            $crate::reader::yaml_parser_update_buffer($parser, __len) != 0
        }
    }};
}

/// Advance the buffer pointer.
#[macro_export]
macro_rules! SKIP {
    ($parser:expr) => {{
        (*$parser).mark.index += 1;
        (*$parser).mark.column += 1;
        (*$parser).unread -= 1;
        (*$parser).buffer.pointer = (*$parser)
            .buffer
            .pointer
            .add($crate::WIDTH!((*$parser).buffer) as usize);
    }};
}

#[macro_export]
macro_rules! SKIP_LINE {
    ($parser:expr) => {{
        if $crate::IS_CRLF!((*$parser).buffer) {
            (*$parser).mark.index += 2;
            (*$parser).mark.column = 0;
            (*$parser).mark.line += 1;
            (*$parser).unread -= 2;
            (*$parser).buffer.pointer = (*$parser).buffer.pointer.add(2);
        } else if $crate::IS_BREAK!((*$parser).buffer) {
            (*$parser).mark.index += 1;
            (*$parser).mark.column = 0;
            (*$parser).mark.line += 1;
            (*$parser).unread -= 1;
            (*$parser).buffer.pointer = (*$parser)
                .buffer
                .pointer
                .add($crate::WIDTH!((*$parser).buffer) as usize);
        }
    }};
}

/// Copy a character to a string buffer and advance pointers.
#[macro_export]
macro_rules! READ {
    ($parser:expr, $string:expr) => {{
        if $crate::STRING_EXTEND!($parser, $string) {
            $crate::COPY!($string, (*$parser).buffer);
            (*$parser).mark.index += 1;
            (*$parser).mark.column += 1;
            (*$parser).unread -= 1;
            true
        } else {
            false
        }
    }};
}

/// Copy a line break character to a string buffer and advance pointers.
#[macro_export]
macro_rules! READ_LINE {
    ($parser:expr, $string:expr) => {{
        if $crate::STRING_EXTEND!($parser, $string) {
            if $crate::CHECK_AT!((*$parser).buffer, b'\r', 0)
                && $crate::CHECK_AT!((*$parser).buffer, b'\n', 1)
            {
                /* CR LF -> LF */
                *$string.pointer = b'\n';
                $string.pointer = $string.pointer.add(1);
                (*$parser).buffer.pointer = (*$parser).buffer.pointer.add(2);
                (*$parser).mark.index += 2;
                (*$parser).mark.column = 0;
                (*$parser).mark.line += 1;
                (*$parser).unread -= 2;
            } else if $crate::CHECK_AT!((*$parser).buffer, b'\r', 0)
                || $crate::CHECK_AT!((*$parser).buffer, b'\n', 0)
            {
                /* CR|LF -> LF */
                *$string.pointer = b'\n';
                $string.pointer = $string.pointer.add(1);
                (*$parser).buffer.pointer = (*$parser).buffer.pointer.add(1);
                (*$parser).mark.index += 1;
                (*$parser).mark.column = 0;
                (*$parser).mark.line += 1;
                (*$parser).unread -= 1;
            } else if $crate::CHECK_AT!((*$parser).buffer, 0xC2u8, 0)
                && $crate::CHECK_AT!((*$parser).buffer, 0x85u8, 1)
            {
                /* NEL -> LF */
                *$string.pointer = b'\n';
                $string.pointer = $string.pointer.add(1);
                (*$parser).buffer.pointer = (*$parser).buffer.pointer.add(2);
                (*$parser).mark.index += 1;
                (*$parser).mark.column = 0;
                (*$parser).mark.line += 1;
                (*$parser).unread -= 1;
            } else if $crate::CHECK_AT!((*$parser).buffer, 0xE2u8, 0)
                && $crate::CHECK_AT!((*$parser).buffer, 0x80u8, 1)
                && ($crate::CHECK_AT!((*$parser).buffer, 0xA8u8, 2)
                    || $crate::CHECK_AT!((*$parser).buffer, 0xA9u8, 2))
            {
                /* LS|PS -> LS|PS */
                let mut __i = 0;
                while __i < 3 {
                    *$string.pointer = *(*$parser).buffer.pointer;
                    $string.pointer = $string.pointer.add(1);
                    (*$parser).buffer.pointer = (*$parser).buffer.pointer.add(1);
                    __i += 1;
                }
                (*$parser).mark.index += 1;
                (*$parser).mark.column = 0;
                (*$parser).mark.line += 1;
                (*$parser).unread -= 1;
            }
            true
        } else {
            false
        }
    }};
}

/* ---------------- parser.c ---------------- */

/// Peek the next token in the token queue. Returns `*mut yaml_token_t`
/// (null on failure).
#[macro_export]
macro_rules! PEEK_TOKEN {
    ($parser:expr) => {{
        if (*$parser).token_available != 0
            || $crate::scanner::yaml_parser_fetch_more_tokens($parser) != 0
        {
            (*$parser).tokens.head
        } else {
            core::ptr::null_mut::<$crate::yaml_token_t>()
        }
    }};
}

/// Remove the next token from the queue (must be called after `PEEK_TOKEN`).
#[macro_export]
macro_rules! SKIP_TOKEN {
    ($parser:expr) => {{
        (*$parser).token_available = 0;
        (*$parser).tokens_parsed += 1;
        (*$parser).stream_end_produced =
            ((*(*$parser).tokens.head).type_ == $crate::YAML_STREAM_END_TOKEN)
                as $crate::libc_shim::c_int;
        (*$parser).tokens.head = (*$parser).tokens.head.add(1);
    }};
}

/* ---------------- emitter.c ---------------- */

/// Flush the buffer if needed.
#[macro_export]
macro_rules! FLUSH {
    ($emitter:expr) => {
        ((*$emitter).buffer.pointer.add(5) < (*$emitter).buffer.end
            || $crate::writer::yaml_emitter_flush($emitter) != 0)
    };
}

/// Put a character to the output buffer.
#[macro_export]
macro_rules! PUT {
    ($emitter:expr, $value:expr) => {{
        if $crate::FLUSH!($emitter) {
            *(*$emitter).buffer.pointer = ($value) as $crate::yaml_char_t;
            (*$emitter).buffer.pointer = (*$emitter).buffer.pointer.add(1);
            (*$emitter).column += 1;
            true
        } else {
            false
        }
    }};
}

/// Put a line break to the output buffer.
#[macro_export]
macro_rules! PUT_BREAK {
    ($emitter:expr) => {{
        if $crate::FLUSH!($emitter) {
            if (*$emitter).line_break == $crate::YAML_CR_BREAK {
                *(*$emitter).buffer.pointer = b'\r';
                (*$emitter).buffer.pointer = (*$emitter).buffer.pointer.add(1);
            } else if (*$emitter).line_break == $crate::YAML_LN_BREAK {
                *(*$emitter).buffer.pointer = b'\n';
                (*$emitter).buffer.pointer = (*$emitter).buffer.pointer.add(1);
            } else if (*$emitter).line_break == $crate::YAML_CRLN_BREAK {
                *(*$emitter).buffer.pointer = b'\r';
                (*$emitter).buffer.pointer = (*$emitter).buffer.pointer.add(1);
                *(*$emitter).buffer.pointer = b'\n';
                (*$emitter).buffer.pointer = (*$emitter).buffer.pointer.add(1);
            }
            (*$emitter).column = 0;
            (*$emitter).line += 1;
            true
        } else {
            false
        }
    }};
}

/// Copy a character from a string into the buffer.
#[macro_export]
macro_rules! WRITE {
    ($emitter:expr, $string:expr) => {{
        if $crate::FLUSH!($emitter) {
            $crate::COPY!((*$emitter).buffer, $string);
            (*$emitter).column += 1;
            true
        } else {
            false
        }
    }};
}

/// Copy a line break character from a string into the buffer.
#[macro_export]
macro_rules! WRITE_BREAK {
    ($emitter:expr, $string:expr) => {{
        if $crate::FLUSH!($emitter) {
            if $crate::CHECK!($string, b'\n') {
                $crate::PUT_BREAK!($emitter);
                $string.pointer = $string.pointer.add(1);
            } else {
                $crate::COPY!((*$emitter).buffer, $string);
                (*$emitter).column = 0;
                (*$emitter).line += 1;
            }
            true
        } else {
            false
        }
    }};
}

/* ---------------------------------------------------------------------- */
/* assert()                                                                */
/* ---------------------------------------------------------------------- */

/// Reproduces glibc's `__assert_fail`: a message on stderr, then `abort()`.
///
/// Note there is deliberately **no** `fflush(stdout)` here.  glibc flushes only
/// `stderr`, and `abort()` does not flush stdio at all, so a C program that
/// trips an assertion LOSES whatever is still sitting in stdout's block
/// buffer.  Flushing would make this port emit a tail the original drops.
///
/// `file`/`func`/`expr` are Rust `&str`s and therefore not NUL-terminated, so
/// they go through `%.*s` rather than `%s`.
#[cold]
#[inline(never)]
pub unsafe fn yaml_assert_fail(expr: &str, file: &str, line: u32, func: &str) -> ! {
    unsafe {
        crate::libc_shim::fprintf(
            crate::libc_shim::stderr,
            b"%s: %.*s:%u: %.*s: Assertion `%.*s' failed.\n\0".as_ptr() as *const c_char,
            crate::libc_shim::__progname,
            file.len() as c_int,
            file.as_ptr() as *const c_char,
            line as c_uint,
            func.len() as c_int,
            func.as_ptr() as *const c_char,
            expr.len() as c_int,
            expr.as_ptr() as *const c_char,
        );
        crate::libc_shim::abort()
    }
}

/// The name of the enclosing function, as C's `__PRETTY_FUNCTION__` would give
/// it to `assert()`.
macro_rules! __yaml_fn_name {
    () => {{
        fn __f() {}
        fn __type_name_of<T>(_: T) -> &'static str {
            ::core::any::type_name::<T>()
        }
        // "libyamlrs::api::yaml_parser_initialize::__f" -> "yaml_parser_initialize"
        let path = __type_name_of(__f);
        let path = &path[..path.len() - "::__f".len()];
        match path.rfind("::") {
            Some(i) => &path[i + 2..],
            None => path,
        }
    }};
}
pub(crate) use __yaml_fn_name;

/// C's `assert()`.
///
/// **These are LIVE, not `debug_assert!`.** libyaml's reference build does not
/// define `NDEBUG`: `c_src/CMakeLists.txt` never sets `CMAKE_BUILD_TYPE` (so
/// CMake adds no `-DNDEBUG`), and `configure.ac` leaves the autotools default
/// `CFLAGS="-g -O2"` alone.  Compiling them out would diverge from the C on
/// every API-misuse path — e.g. `yaml_alias_event_initialize(event, NULL)`
/// aborts with status 134 in C, but would reach `strlen(NULL)` and segfault
/// with status 139 here.
///
/// `assert(0)` therefore really does abort, and `assert(1)` really is a no-op.
macro_rules! yaml_assert {
    ($cond:expr) => {{
        if !($cond) {
            unsafe {
                $crate::private::yaml_assert_fail(
                    stringify!($cond),
                    file!(),
                    line!(),
                    $crate::private::__yaml_fn_name!(),
                )
            }
        }
    }};
}
pub(crate) use yaml_assert;
