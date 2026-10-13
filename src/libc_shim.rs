//! Hand-rolled declarations of the C library facilities that libyaml uses.
//!
//! The provided `Cargo.toml` has no `[dependencies]` section and must not be
//! modified, so the `libc` crate is unavailable.  A `cdylib` on Linux links
//! against the system C library anyway, so declaring the symbols directly here
//! resolves at link time and keeps `yaml_malloc`/`yaml_free` interoperable with
//! C callers (which is required: C code frees strings that the library
//! allocated, and the library frees strings produced by `strdup`).

pub use core::ffi::c_char;
pub use core::ffi::c_int;
pub use core::ffi::c_long;
pub use core::ffi::c_uchar;
pub use core::ffi::c_uint;
pub use core::ffi::c_void;

/// `size_t`
pub type size_t = usize;
/// `ptrdiff_t`
pub type ptrdiff_t = isize;

/// `INT_MAX` from `<limits.h>` (C `int` is 32-bit on every platform we target).
pub const INT_MAX: c_int = c_int::MAX;

/// Opaque `FILE`.
#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub fn malloc(size: size_t) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn strdup(s: *const c_char) -> *mut c_char;

    pub fn memset(s: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn memcpy(dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memmove(dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memcmp(s1: *const c_void, s2: *const c_void, n: size_t) -> c_int;

    pub fn strlen(s: *const c_char) -> size_t;
    pub fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    pub fn strncmp(s1: *const c_char, s2: *const c_char, n: size_t) -> c_int;

    pub fn fread(ptr: *mut c_void, size: size_t, nmemb: size_t, stream: *mut FILE) -> size_t;
    pub fn fwrite(ptr: *const c_void, size: size_t, nmemb: size_t, stream: *mut FILE) -> size_t;
    pub fn fflush(stream: *mut FILE) -> c_int;
    pub fn ferror(stream: *mut FILE) -> c_int;

    /* Used only by `yaml_assert!` (see `private.rs`) to reproduce glibc's
     * `__assert_fail`.  libyaml's `assert()`s are live in the reference build:
     * neither `c_src/CMakeLists.txt` (which never sets `CMAKE_BUILD_TYPE`) nor
     * `configure.ac` defines `NDEBUG`. */
    pub static mut stderr: *mut FILE;
    /// The basename of `argv[0]` — what glibc's `__assert_fail` prefixes its
    /// message with (*not* `program_invocation_name`).
    pub static mut __progname: *mut c_char;
    pub fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn abort() -> !;
}
