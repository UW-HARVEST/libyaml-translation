# Contract for translating `c_src/tests/*.c` into `src/bin/*.rs`

Read this in full before writing a line of Rust. It is the authoritative spec
for the 14 consumer programs in `c_src/tests/`. (The *library* itself —
`c_src/src/*.c` — is already translated under `src/`; do **not** touch it, and
do not touch `Cargo.toml`, `build.rs`, `rust-toolchain.toml`, `PLAN.md` or
anything under `c_src/`.)

## 1. Why these are `src/bin/*.rs`

`Cargo.toml` is provided and declares only `[lib] crate-type = ["cdylib"]`. It
must not be edited. Cargo nevertheless **auto-discovers** every `src/bin/*.rs`
as a binary target, so each C test program becomes one file there. The binary
target name is the file stem, so the names must match the CMake targets in
`c_src/tests/CMakeLists.txt` **exactly**, hyphens included:

```
src/bin/example-deconstructor.rs      src/bin/run-loader.rs
src/bin/example-deconstructor-alt.rs  src/bin/run-parser.rs
src/bin/example-reformatter.rs        src/bin/run-parser-test-suite.rs
src/bin/example-reformatter-alt.rs    src/bin/run-scanner.rs
src/bin/run-dumper.rs                 src/bin/test-nesting.rs
src/bin/run-emitter.rs                src/bin/test-reader.rs
src/bin/run-emitter-test-suite.rs     src/bin/test-version.rs
```

A `cdylib` cannot be linked as a Rust dependency, so each binary pulls the
library in **as a module** instead, via `#[path = "../lib.rs"]`.

## 2. Mandatory file preamble

Copy this verbatim at the top of every `src/bin/*.rs`, then add whatever extra
`use crate::<module>::*;` lines the program needs:

```rust
//! Translation of `c_src/tests/<NAME>.c`.

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

use crate::ctest::*;
```

`use yamllib::*;` makes the library's modules reachable as `crate::api`,
`crate::parser`, … and brings every `yaml.h` type/constant
(`yaml_event_t`, `YAML_SCALAR_EVENT`, `yaml_char_t`, …) into scope.

`src/bin/run-scanner.rs` is already written and is the reference example —
**read it first**; it shows the whole pattern end to end.

### Where the library functions live

| `use crate::…::*;`     | functions |
|---|---|
| `api`            | `yaml_parser_initialize/delete`, `yaml_parser_set_input_string/_file/_encoding`, `yaml_emitter_initialize/delete`, `yaml_emitter_set_output_string/_file/_canonical/_indent/_width/_unicode/_break` |
| `api_objects`    | `yaml_token_delete`, every `yaml_*_event_initialize`, `yaml_event_delete` |
| `api_document`   | `yaml_document_initialize/delete/get_node/get_root_node/add_scalar/add_sequence/add_mapping/append_sequence_item/append_mapping_pair` |
| `scanner`        | `yaml_parser_scan` |
| `parser`         | `yaml_parser_parse`, `yaml_set_max_nest_level`, `yaml_get_max_nest_level` |
| `loader`         | `yaml_parser_load` |
| `emitter`        | `yaml_emitter_emit` |
| `writer`         | `yaml_emitter_flush` |
| `dumper`         | `yaml_emitter_open/close/dump` |
| `reader`         | `yaml_parser_update_buffer` |
| `memory`         | `yaml_get_version`, `yaml_get_version_string`, `yaml_malloc`, `yaml_free`, … |

## 3. `src/ctest.rs` — the C runtime shim

Already written. `use crate::ctest::*;` gives you:

- **Types**: `c_char`, `c_int`, `c_long`, `c_uchar`, `c_uint`, `c_void`,
  `size_t`, `FILE`, `INT_MAX`.
- **Streams**: `stdin`, `stdout`, `stderr` (all `*mut FILE`, read inside `unsafe`).
- **stdio**: `fopen`, `fclose`, `feof`, `ferror`, `fflush`, `fgets`, `fputs`,
  `fputc`, `putchar`, `fread`, `fwrite`, `printf`, `fprintf`, `sprintf`,
  `snprintf`.
- **string/stdlib**: `strlen`, `strcmp`, `strncmp`, `strchr`, `strrchr`,
  `strtol`, `strdup`, `memset`, `memcpy`, `memmove`, `memcmp`, `malloc`,
  `realloc`, `free`, `exit`, `abort`.
- **Macros**: `cstr!("…")` → `*const c_char`; `c_assert!(expr)` → C `assert()`.
- **Entry points**: `ctest::run(c_main)` for `int main(int, char**)`;
  `ctest::run_void(c_main)` for `int main(void)`.

### Use the *C* stdio, never Rust's

Do **not** use `println!`, `print!`, `eprintln!`, `std::io::stdout()` or
`std::process::exit`. The programs' observable behaviour includes C stdio
buffering: C's `stdout` is fully buffered when it is not a tty, Rust's is always
line buffered, so mixing them reorders stdout against stderr under a pipe.
`ctest::run` leaves through C's `exit()`, which flushes the C buffers exactly
like the original.

## 4. Translation rules

1. **`assert()` is live.** Every test file does
   `#ifdef NDEBUG / #undef NDEBUG / #endif` before `#include <assert.h>`, so
   assertions are enabled regardless of build flags and the argument is
   **always evaluated**. Several tests rely on that: e.g. `run-dumper.c` has
   `assert(yaml_parser_load(&parser, &document) || print_output(...));`.
   Translate `assert(x)` as `c_assert!(x)` — never as `debug_assert!`, never by
   dropping it. `c_assert!` accepts C-truthy values: `bool`, any integer, any
   raw pointer.

2. **String literals** become `cstr!("…")`. Keep escapes exactly
   (`\n`, `\"`, `\\`); Rust and C agree on these. `printf` format strings pass
   through unchanged — they are handed to the real C `printf`.

3. **Variadic argument types must be pre-promoted**, because Rust does no
   default argument promotion for C variadics:

   | format | pass |
   |---|---|
   | `%d` / `%i` | `c_int` |
   | `%u` / `%X` / `%x` | `c_uint` |
   | `%ld` / `%lu` | `c_long` / `u64` (cast with `as c_long` / `as u64`) |
   | `%zu` / `%zd` | `size_t` (= `usize`) |
   | `%s` | `*const c_char` (cast `*mut yaml_char_t` with `as *const c_char`) |
   | `%c` | `c_int` |

   `sizeof(T)` → `core::mem::size_of::<T>()`; for `printf("%ld", (long)sizeof(T))`
   write `core::mem::size_of::<T>() as c_long`.

4. **Field renames.** The C field `type` is `type_` in Rust
   (`event.type_`, `token.type_`, `node.type_`). Everything else keeps its C
   name. Unions are real `#[repr(C)] union`s, so reading them needs `unsafe`
   and the variant name: `event.data.scalar.value`,
   `event.data.document_start.version_directive`, …

5. **Uninitialized locals.** C declares `yaml_parser_t parser;` uninitialized.
   Write `let mut parser: yaml_parser_t = core::mem::zeroed();` — never
   `MaybeUninit::assume_init()`, which is UB. Every one of these structs is
   either `memset` to 0 or fully initialized by the library before it is read,
   so zeroing is behaviourally identical.
   `memset(&parser, 0, sizeof(parser))` →
   `memset(&mut parser as *mut _ as *mut c_void, 0, core::mem::size_of::<yaml_parser_t>());`

6. **`char` is signed** on x86-64 Linux: `char buf[1024]` → `[c_char; 1024]`
   (`c_char` = `i8`). Comparing against a character literal needs
   `b'x' as c_char`. Index with `as usize`. Take a pointer with
   `buf.as_mut_ptr()`.

7. **`goto`** → a labeled block (`'error: { … break 'error; … }`) or a labeled
   loop, following the structure already used throughout `src/*.rs` (see
   `src/scanner_scalar.rs` for the `goto error` idiom). Keep the C's statement
   order and the exact cleanup sequence.

8. **Reproduce behaviour, do not improve it.** Same order of error checks, same
   messages, same spacing and newlines, same return codes. Do not fix bugs, do
   not add bounds checks the C does not have, do not reorder `fflush` calls.

9. **Whole-body `unsafe`.** Write `unsafe fn c_main(...) -> c_int { unsafe { … } }`
   and keep helper functions `unsafe fn` too. The preamble already silences
   `unused_unsafe`.

10. **`main`**:
    ```rust
    fn main() { ctest::run(c_main) }        // int main(int argc, char *argv[])
    fn main() { ctest::run_void(c_main) }   // int main(void)
    ```
    `argv` is `*mut *mut c_char`; `argv[k]` is `*argv.offset(k as isize)`.

## 5. Verifying your work

```bash
cd /tmp/.tmpyMrUvL/translated_rust
cargo build --release --bin <NAME>          # must be clean; cargo may briefly
                                            # block on the target-dir lock
cp target/release/<NAME> /tmp/ref/R/
/tmp/ref/cmp.sh <NAME> [args...]
```

`/tmp/ref/C/<NAME>` is the original C program compiled against the real libyaml;
`/tmp/ref/cmp.sh` runs both and diffs stdout, stderr and the exit status,
printing `OK: …` when they agree. Use **absolute** paths for file arguments
(the two programs run from different directories). Good inputs:
`/tmp/.tmpyMrUvL/translated_rust/c_src/examples/*.yaml` and
`/tmp/.tmpyMrUvL/translated_rust/c_src/regression-inputs/*`. For the programs
that read stdin, pipe the same file into both — `cmp.sh` does not do stdin, so
run them by hand:

```bash
/tmp/ref/C/<NAME> < in > /tmp/c.out 2>/tmp/c.err; echo $?
/tmp/ref/R/<NAME> < in > /tmp/r.out 2>/tmp/r.err; echo $?
diff /tmp/c.out /tmp/r.out && diff /tmp/c.err /tmp/r.err
```

The only tolerated difference is the `file:line` inside a glibc
assertion-failure message (`<prog>: <file>:<line>: main: Assertion \`x' failed.`),
since `__FILE__`/`__LINE__` necessarily refer to the Rust source. Everything
else must match byte for byte.

## 6. Reporting back

State which files you created, whether `cargo build --release --bin <NAME>`
is clean, what you compared against the C reference and the result, and any
pitfall worth recording.
