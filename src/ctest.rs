//! Shared C-runtime shim for the translated `c_src/tests/*.c` consumer programs.
//!
//! The test programs in `c_src/tests/` are ordinary C programs that link against
//! libyaml.  They use `<stdio.h>`, `<stdlib.h>`, `<string.h>` and `<assert.h>`
//! directly, and their observable behaviour (buffering, interleaving of stdout
//! and stderr, flush-on-exit) is C-stdio behaviour.  Reproducing that exactly
//! with Rust's `std::io::Stdout` is impossible — Rust's stdout is always
//! line-buffered, whereas C's is fully buffered when it is not a tty — so the
//! translations call the real C stdio through these declarations instead.
//!
//! This file is **not** part of the `libyamlrs` cdylib: `src/lib.rs` does not
//! declare it.  Each binary under `src/bin/` pulls it in with
//!
//! ```ignore
//! #[macro_use]
//! #[path = "../ctest.rs"]
//! mod ctest;
//! ```

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_macros)]
#![allow(unused_unsafe)]

pub use crate::libc_shim::{
    c_char, c_int, c_long, c_uchar, c_uint, c_void, ptrdiff_t, size_t, FILE, INT_MAX,
};

// Re-export the libc facilities the library shim already declares, so a test
// program only needs `use crate::ctest::*;`.
pub use crate::libc_shim::{
    ferror, fflush, fread, free, fwrite, malloc, memcmp, memcpy, memmove, memset, realloc, strcmp,
    strdup, strlen, strncmp,
};

unsafe extern "C" {
    pub static mut stdin: *mut FILE;
    pub static mut stdout: *mut FILE;
    pub static mut stderr: *mut FILE;

    /// glibc's `argv[0]`.
    pub static mut program_invocation_name: *mut c_char;
    /// The basename of `argv[0]`; this is what glibc's `__assert_fail` prefixes
    /// its message with (*not* `program_invocation_name`).
    pub static mut __progname: *mut c_char;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn feof(stream: *mut FILE) -> c_int;
    pub fn fgets(s: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
    pub fn fputs(s: *const c_char, stream: *mut FILE) -> c_int;
    pub fn fputc(c: c_int, stream: *mut FILE) -> c_int;
    pub fn putchar(c: c_int) -> c_int;

    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: size_t, fmt: *const c_char, ...) -> c_int;

    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;

    pub fn exit(status: c_int) -> !;
    pub fn abort() -> !;
}

/// `expr` in a C truth context (`if (expr)`, `assert(expr)`, `!expr`, ...).
///
/// C applies this conversion implicitly to every scalar type; Rust does not, so
/// the `c_assert!` macro routes its argument through this trait.
pub trait CTruthy {
    fn c_is_true(self) -> bool;
}

macro_rules! impl_truthy_int {
    ($($t:ty),*) => {
        $(impl CTruthy for $t {
            #[inline]
            fn c_is_true(self) -> bool { self != 0 }
        })*
    };
}

impl_truthy_int!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);

impl CTruthy for bool {
    #[inline]
    fn c_is_true(self) -> bool {
        self
    }
}

impl<T: ?Sized> CTruthy for *const T {
    #[inline]
    fn c_is_true(self) -> bool {
        !self.is_null()
    }
}

impl<T: ?Sized> CTruthy for *mut T {
    #[inline]
    fn c_is_true(self) -> bool {
        !self.is_null()
    }
}

/// A NUL-terminated C string literal: `cstr!("foo")` has type `*const c_char`.
#[macro_export]
macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const ::core::ffi::c_char
    };
}

/// A C `assert()`.
///
/// Every `c_src/tests/*.c` file does
///
/// ```c
/// #ifdef NDEBUG
/// #undef NDEBUG
/// #endif
/// #include <assert.h>
/// ```
///
/// so the assertions are live no matter how the program is compiled, and the
/// argument expression is **always** evaluated.  That matters: several tests
/// put calls with side effects inside `assert()` (e.g. `run-dumper.c`'s
/// `assert(yaml_parser_load(&parser, &document) || print_output(...))`).
#[macro_export]
macro_rules! c_assert {
    ($cond:expr) => {{
        if !$crate::ctest::CTruthy::c_is_true($cond) {
            unsafe {
                $crate::ctest::assert_fail(
                    cstr!(stringify!($cond)),
                    cstr!(file!()),
                    line!(),
                    cstr!("main"),
                )
            }
        }
    }};
    ($cond:expr, $func:expr) => {{
        if !$crate::ctest::CTruthy::c_is_true($cond) {
            unsafe {
                $crate::ctest::assert_fail(
                    cstr!(stringify!($cond)),
                    cstr!(file!()),
                    line!(),
                    cstr!($func),
                )
            }
        }
    }};
}

/// Reproduces glibc's `__assert_fail`: a message on stderr, then `abort()`.
pub unsafe fn assert_fail(
    expr: *const c_char,
    file: *const c_char,
    line: u32,
    func: *const c_char,
) -> ! {
    unsafe {
        // Deliberately NO `fflush(stdout)` here.  glibc's `__assert_fail`
        // flushes only `stderr`, and `abort()` does not flush stdio at all, so
        // a C program that aborts LOSES whatever is still sitting in stdout's
        // block buffer.  Verified empirically: a program that does
        // `printf("...no newline..."); assert(0);` with stdout on a pipe
        // produces 0 bytes of stdout.  Flushing here would make the Rust
        // translations emit a tail the originals drop.
        fprintf(
            stderr,
            cstr!("%s: %s:%u: %s: Assertion `%s' failed.\n"),
            __progname,
            file,
            line as c_uint,
            func,
            expr,
        );
        abort()
    }
}

/// Rebuilds C's `argv`: a NUL-terminated array of NUL-terminated byte strings.
///
/// `std::env::args()` would panic on non-UTF-8 arguments, which C happily
/// passes through, so this goes through `args_os()` and the raw bytes.  The
/// allocations are leaked on purpose — in C they live in the initial process
/// stack for the whole run.
pub fn c_args() -> (c_int, *mut *mut c_char) {
    use std::os::unix::ffi::OsStrExt;

    let mut ptrs: Vec<*mut c_char> = Vec::new();
    for arg in std::env::args_os() {
        let bytes = arg.as_bytes();
        let mut owned: Vec<u8> = Vec::with_capacity(bytes.len() + 1);
        owned.extend_from_slice(bytes);
        owned.push(0);
        ptrs.push(owned.leak().as_mut_ptr() as *mut c_char);
    }
    let argc = ptrs.len() as c_int;
    ptrs.push(core::ptr::null_mut());
    let argv = ptrs.leak().as_mut_ptr();
    (argc, argv)
}

/// Entry point shim: run the translated `main`, then leave through C's `exit()`
/// so that the C stdio buffers are flushed exactly as they would be in the
/// original program.
pub fn run(main: unsafe fn(c_int, *mut *mut c_char) -> c_int) -> ! {
    let (argc, argv) = c_args();
    let status = unsafe { main(argc, argv) };
    unsafe { exit(status) }
}

/// `run()` for the `int main(void)` programs.
pub fn run_void(main: unsafe fn() -> c_int) -> ! {
    let status = unsafe { main() };
    unsafe { exit(status) }
}
