# Translation contract — read this before writing any Rust

You are translating **libyaml 0.2.5** (C, in `c_src/`) into pure Rust in
`/tmp/.tmpyMrUvL/translated_rust/src/`. The crate is `libyamlrs`, built as a
**`cdylib`**, so C programs link against it and the C ABI must be exact.

## Absolute rules

1. **Do NOT modify** `Cargo.toml`, `rust-toolchain.toml`, `PLAN.md`,
   `TRANSLATION_CONTRACT.md`, `src/lib.rs`, `src/private.rs`, `src/memory.rs`,
   `src/libc_shim.rs`, or anything in `c_src/`. Write ONLY the file(s) your
   task assigns you. (If you believe a shared file is missing something, say so
   in your final report instead of editing it.)
2. **Do NOT add dependencies.** There is no `libc` crate. Everything you need
   from C is already declared in `src/libc_shim.rs`.
3. **Faithful translation, no "improvements".** Do not fix bugs, do not reorder
   checks, do not change error strings, do not change control flow. Byte-identical
   behaviour is the goal. Keep the original C comments.
4. **No stubs.** Every function in your assigned range must be fully implemented.
5. Keep the C function names **verbatim**.
   * C `YAML_DECLARE(...)` (non-static) functions →
     `#[unsafe(no_mangle)] pub unsafe extern "C" fn name(...) -> ...`
   * C `static` functions → `pub(crate) unsafe fn name(...) -> ...`
     (plain Rust fn, NOT `extern "C"`, NOT `no_mangle`; `pub(crate)` so other
     modules in the split can call them).
6. `assert(x)` in C → `debug_assert!(x)` in Rust (release builds define NDEBUG).
7. C `goto error;` → translate with a labelled block or by inlining the cleanup;
   a common faithful shape is:
   ```rust
   'error: {
       ... if cond { break 'error; } ...
       return 1;          // success path returns before the cleanup
   }
   /* error: */
   ...cleanup...
   return 0;
   ```
   Pick whatever is clearest, but the *observable behaviour* must match.

## Rust 2021 / rustc 1.93 details

* `unsafe extern "C" { ... }` for extern blocks, `#[unsafe(no_mangle)]` for exports.
* `&raw mut expr` / `&raw const expr` for raw pointers to places.
* Raw pointer arithmetic: `p.add(n)`, `p.sub(n)`, `p.offset(i)`,
  `a.offset_from(b)` (returns `isize`).
* `core::ptr::null_mut()`, `core::ptr::null()`.

## Types you already have (`src/lib.rs`)

All public types are `#[repr(C)]` and layout-verified against gcc. Key points:

* C enums are **`c_uint` type aliases + `pub const`s**, e.g.
  `pub type yaml_token_type_t = c_uint; pub const YAML_SCALAR_TOKEN: yaml_token_type_t = 21;`
  So a C `switch (token->type)` becomes
  `match (*token).type_ { YAML_SCALAR_TOKEN => {...}, _ => {} }`.
* The C struct field `type` is named **`type_`** in Rust (it's a keyword).
  This applies to `yaml_token_t`, `yaml_event_t`, `yaml_node_t`.
* Anonymous C structs got names:
  `yaml_buffer_t` (start/end/pointer/last), `yaml_stack_t<T>` (start/end/top),
  `yaml_queue_t<T>` (start/end/head/tail),
  `yaml_tag_directives_range_t` (start/end),
  `yaml_parser_input_t` (union: `.string` / `.file`),
  `yaml_emitter_output_t` (union: `.string` / `.file`),
  `yaml_emitter_anchor_data_t`, `yaml_emitter_tag_data_t`,
  `yaml_emitter_scalar_data_t`,
  `yaml_token_data_t`, `yaml_event_data_t`, `yaml_node_data_t` (unions with the
  same member names as C: `.scalar`, `.alias`, `.anchor`, `.tag`,
  `.stream_start`, `.document_start`, `.document_end`, `.sequence_start`,
  `.mapping_start`, `.version_directive`, `.tag_directive`, `.sequence`,
  `.mapping`).
* `read_handler` / `write_handler` fields are `Option<yaml_read_handler_t>` /
  `Option<yaml_write_handler_t>`. Test with `.is_none()` / `.is_some()`, set with
  `Some(f)` / `None`.
* Tag string constants are `&[u8]` with a trailing NUL, e.g.
  `YAML_DEFAULT_SCALAR_TAG` → use `YAML_DEFAULT_SCALAR_TAG.as_ptr() as *mut yaml_char_t`
  where C writes `(yaml_char_t *)YAML_DEFAULT_SCALAR_TAG`.
  (Careful: such pointers must never be freed — matching C, where they are
  always `yaml_strdup`'d before being stored.)
* C string literals used as `const char *problem` messages become
  `b"...\0".as_ptr() as *const c_char`.
* `yaml_mark_t::zeroed()`, `yaml_stack_t::zeroed()`, `yaml_queue_t::zeroed()`,
  `yaml_buffer_t::zeroed()` exist for convenience.

## libc (`src/libc_shim.rs`)

```rust
use crate::libc_shim::{c_char, c_int, c_uchar, c_uint, c_void, size_t, ptrdiff_t,
                       INT_MAX, FILE,
                       malloc, realloc, free, strdup,
                       memset, memcpy, memmove, memcmp,
                       strlen, strcmp, strncmp,
                       fread, fwrite, fflush, ferror};
```

## Memory helpers (`src/memory.rs`) — already written, just call them

`yaml_malloc`, `yaml_realloc`, `yaml_free`, `yaml_strdup`, `yaml_string_extend`,
`yaml_string_join`, `yaml_stack_extend`, `yaml_queue_extend`,
`yaml_get_version`, `yaml_get_version_string`.

Reach them as `crate::memory::yaml_malloc(...)` etc.

## Macros (`src/private.rs`) — already written, just use them

They are `#[macro_export]`ed, so inside the crate call them as
`crate::NAME!(...)` or just `NAME!(...)` (both work; `crate::` is safest).

**They return `bool`, not `int`.** So C's
`if (!PUSH(parser, parser->indents, x)) return 0;`
becomes
`if !PUSH!(parser, (*parser).indents, x) { return 0; }`.

Available, with the same argument order as C:

| group | macros |
|---|---|
| buffer | `BUFFER_INIT!(ctx, buf, size)`, `BUFFER_DEL!(ctx, buf)` |
| string | `NULL_STRING!()`, `STRING!(ptr, len)`, `STRING_ASSIGN!(val, ptr, len)`, `STRING_INIT!(ctx, s, size)`, `STRING_DEL!(ctx, s)`, `STRING_EXTEND!(ctx, s)`, `CLEAR!(ctx, s)`, `JOIN!(ctx, a, b)` |
| char class | `CHECK_AT!`, `CHECK!`, `AT!(s, off)`, `IS_ALPHA_AT!`, `IS_ALPHA!`, `IS_DIGIT_AT!`, `IS_DIGIT!`, `AS_DIGIT_AT!`, `AS_DIGIT!`, `IS_HEX_AT!`, `IS_HEX!`, `AS_HEX_AT!`, `AS_HEX!`, `IS_ASCII_AT!`, `IS_ASCII!`, `IS_PRINTABLE_AT!`, `IS_PRINTABLE!`, `IS_Z_AT!`, `IS_Z!`, `IS_BOM_AT!`, `IS_BOM!`, `IS_SPACE_AT!`, `IS_SPACE!`, `IS_TAB_AT!`, `IS_TAB!`, `IS_BLANK_AT!`, `IS_BLANK!`, `IS_BREAK_AT!`, `IS_BREAK!`, `IS_CRLF_AT!`, `IS_CRLF!`, `IS_BREAKZ_AT!`, `IS_BREAKZ!`, `IS_SPACEZ_AT!`, `IS_SPACEZ!`, `IS_BLANKZ_AT!`, `IS_BLANKZ!`, `WIDTH_AT!`, `WIDTH!`, `MOVE!`, `COPY!(a, b)` |
| stack | `STACK_INIT!(ctx, stack)` *(C's trailing type arg is dropped)*, `STACK_DEL!`, `STACK_EMPTY!`, `STACK_LIMIT!(ctx, stack, size)`, `PUSH!(ctx, stack, value)`, `POP!(ctx, stack)` |
| queue | `QUEUE_INIT!(ctx, queue, size)` *(type arg dropped)*, `QUEUE_DEL!`, `QUEUE_EMPTY!`, `ENQUEUE!(ctx, q, value)`, `DEQUEUE!(ctx, q)`, `QUEUE_INSERT!(ctx, q, index, value)` |
| token init | `TOKEN_INIT!`, `STREAM_START_TOKEN_INIT!`, `STREAM_END_TOKEN_INIT!`, `ALIAS_TOKEN_INIT!`, `ANCHOR_TOKEN_INIT!`, `TAG_TOKEN_INIT!`, `SCALAR_TOKEN_INIT!`, `VERSION_DIRECTIVE_TOKEN_INIT!`, `TAG_DIRECTIVE_TOKEN_INIT!` |
| event init | `EVENT_INIT!`, `STREAM_START_EVENT_INIT!`, `STREAM_END_EVENT_INIT!`, `DOCUMENT_START_EVENT_INIT!`, `DOCUMENT_END_EVENT_INIT!`, `ALIAS_EVENT_INIT!`, `SCALAR_EVENT_INIT!`, `SEQUENCE_START_EVENT_INIT!`, `SEQUENCE_END_EVENT_INIT!`, `MAPPING_START_EVENT_INIT!`, `MAPPING_END_EVENT_INIT!` |
| doc/node init | `DOCUMENT_INIT!`, `NODE_INIT!`, `SCALAR_NODE_INIT!`, `SEQUENCE_NODE_INIT!`, `MAPPING_NODE_INIT!` |
| scanner.c | `CACHE!(parser, len)`, `SKIP!(parser)`, `SKIP_LINE!(parser)`, `READ!(parser, string)`, `READ_LINE!(parser, string)` |
| parser.c | `PEEK_TOKEN!(parser)` → `*mut yaml_token_t`, `SKIP_TOKEN!(parser)` |
| emitter.c | `FLUSH!(em)`, `PUT!(em, value)`, `PUT_BREAK!(em)`, `WRITE!(em, string)`, `WRITE_BREAK!(em, string)` |

Also `pub static mut MAX_NESTING_LEVEL: c_int` lives in `crate::private`
(exported symbol `MAX_NESTING_LEVEL`). Read it as
`unsafe { crate::private::MAX_NESTING_LEVEL }`.

Size constants in `crate::private`: `INPUT_RAW_BUFFER_SIZE`, `INPUT_BUFFER_SIZE`,
`OUTPUT_BUFFER_SIZE`, `OUTPUT_RAW_BUFFER_SIZE`, `MAX_FILE_SIZE`,
`INITIAL_STACK_SIZE`, `INITIAL_QUEUE_SIZE`, `INITIAL_STRING_SIZE`, and the type
`yaml_string_t { start, end, pointer }`.

### Macro call-site conventions (important)

The macros do field access on whatever you pass, exactly like C. Pass an
**lvalue**:

```rust
// C: BUFFER_INIT(parser, parser->raw_buffer, INPUT_RAW_BUFFER_SIZE)
if !crate::BUFFER_INIT!(parser, (*parser).raw_buffer, crate::private::INPUT_RAW_BUFFER_SIZE) { ... }

// C: CHECK(parser->buffer, '-')
if crate::CHECK!((*parser).buffer, b'-') { ... }

// C: PUSH(parser, parser->indents, parser->indent)
if !crate::PUSH!(parser, (*parser).indents, (*parser).indent) { ... }

// C: yaml_token_delete(&DEQUEUE(parser, parser->tokens));
let mut __t = crate::DEQUEUE!(parser, (*parser).tokens);
crate::api_objects::yaml_token_delete(&mut __t);
```

The `ctx`/`parser`/`emitter` first argument is the raw **pointer** (the macro does
`(*$ctx).error = ...` internally), so pass `parser`, not `(*parser)`.

## Module map (who owns what)

| Rust module | C origin |
|---|---|
| `src/memory.rs` | `api.c` 1–162 (done) |
| `src/api.rs` | `api.c` 164–582 |
| `src/api_objects.rs` | `api.c` 583–1033 |
| `src/api_document.rs` | `api.c` 1035–1393 |
| `src/reader.rs` | `reader.c` |
| `src/writer.rs` | `writer.c` |
| `src/scanner.rs` | `scanner.c` 744–1309 |
| `src/scanner_tokens.rs` | `scanner.c` 1310–1937 |
| `src/scanner_scan.rs` | `scanner.c` 1938–2758 |
| `src/scanner_scalar.rs` | `scanner.c` 2759–3616 |
| `src/parser.rs` | `parser.c` 166–743 |
| `src/parser_states.rs` | `parser.c` 744–1398 |
| `src/loader.rs` | `loader.c` |
| `src/dumper.rs` | `dumper.c` |
| `src/emitter.rs` | `emitter.c` 264–1108 |
| `src/emitter_analyze.rs` | `emitter.c` 1109–1775 |
| `src/emitter_write.rs` | `emitter.c` 1776–2358 |

Cross-module calls use the module path, e.g.
`crate::scanner::yaml_parser_fetch_more_tokens(parser)`,
`crate::emitter_write::yaml_emitter_write_indent(emitter)`,
`crate::emitter_analyze::yaml_emitter_analyze_event(emitter, event)`,
`crate::api_objects::yaml_event_delete(event)`,
`crate::api_document::yaml_document_delete(doc)`,
`crate::reader::yaml_parser_update_buffer(parser, len)`,
`crate::writer::yaml_emitter_flush(emitter)`.

Functions you need that live in *another* agent's module may not exist yet when
you write your file. That is fine — write the call with the correct path and
signature anyway; the main agent compiles everything at the end.

## Signature reference for cross-module calls

```text
reader.rs       yaml_parser_update_buffer(parser: *mut yaml_parser_t, length: size_t) -> c_int   [no_mangle extern "C"]
writer.rs       yaml_emitter_flush(emitter: *mut yaml_emitter_t) -> c_int                        [no_mangle extern "C"]
scanner.rs      yaml_parser_fetch_more_tokens(parser: *mut yaml_parser_t) -> c_int               [no_mangle extern "C"]
scanner.rs      yaml_parser_scan(parser: *mut yaml_parser_t, token: *mut yaml_token_t) -> c_int  [no_mangle extern "C"]
scanner.rs      yaml_parser_set_scanner_error(parser, context: *const c_char, context_mark: yaml_mark_t, problem: *const c_char) -> c_int   [pub(crate)]
scanner.rs      yaml_parser_roll_indent(parser, column: ptrdiff_t, number: ptrdiff_t, type_: yaml_token_type_t, mark: yaml_mark_t) -> c_int [pub(crate)]
scanner.rs      yaml_parser_unroll_indent(parser, column: ptrdiff_t) -> c_int                    [pub(crate)]
scanner.rs      yaml_parser_save_simple_key(parser) -> c_int                                     [pub(crate)]
scanner.rs      yaml_parser_remove_simple_key(parser) -> c_int                                   [pub(crate)]
scanner.rs      yaml_parser_increase_flow_level(parser) -> c_int                                 [pub(crate)]
scanner.rs      yaml_parser_decrease_flow_level(parser) -> c_int                                 [pub(crate)]
scanner.rs      yaml_parser_stale_simple_keys(parser) -> c_int                                   [pub(crate)]
scanner.rs      yaml_parser_fetch_next_token(parser) -> c_int                                    [pub(crate)]
scanner_tokens.rs  yaml_parser_fetch_stream_start(parser) -> c_int
scanner_tokens.rs  yaml_parser_fetch_stream_end(parser) -> c_int
scanner_tokens.rs  yaml_parser_fetch_directive(parser) -> c_int
scanner_tokens.rs  yaml_parser_fetch_document_indicator(parser, type_: yaml_token_type_t) -> c_int
scanner_tokens.rs  yaml_parser_fetch_flow_collection_start(parser, type_: yaml_token_type_t) -> c_int
scanner_tokens.rs  yaml_parser_fetch_flow_collection_end(parser, type_: yaml_token_type_t) -> c_int
scanner_tokens.rs  yaml_parser_fetch_flow_entry(parser) -> c_int
scanner_tokens.rs  yaml_parser_fetch_block_entry(parser) -> c_int
scanner_tokens.rs  yaml_parser_fetch_key(parser) -> c_int
scanner_tokens.rs  yaml_parser_fetch_value(parser) -> c_int
scanner_tokens.rs  yaml_parser_fetch_anchor(parser, type_: yaml_token_type_t) -> c_int
scanner_tokens.rs  yaml_parser_fetch_tag(parser) -> c_int
scanner_tokens.rs  yaml_parser_fetch_block_scalar(parser, literal: c_int) -> c_int
scanner_tokens.rs  yaml_parser_fetch_flow_scalar(parser, single: c_int) -> c_int
scanner_tokens.rs  yaml_parser_fetch_plain_scalar(parser) -> c_int
scanner_scan.rs    yaml_parser_scan_to_next_token(parser) -> c_int
scanner_scan.rs    yaml_parser_scan_directive(parser, token: *mut yaml_token_t) -> c_int
scanner_scan.rs    yaml_parser_scan_directive_name(parser, start_mark: yaml_mark_t, name: *mut *mut yaml_char_t) -> c_int
scanner_scan.rs    yaml_parser_scan_version_directive_value(parser, start_mark: yaml_mark_t, major: *mut c_int, minor: *mut c_int) -> c_int
scanner_scan.rs    yaml_parser_scan_version_directive_number(parser, start_mark: yaml_mark_t, number: *mut c_int) -> c_int
scanner_scan.rs    yaml_parser_scan_tag_directive_value(parser, mark: yaml_mark_t, handle: *mut *mut yaml_char_t, prefix: *mut *mut yaml_char_t) -> c_int
scanner_scan.rs    yaml_parser_scan_anchor(parser, token: *mut yaml_token_t, type_: yaml_token_type_t) -> c_int
scanner_scan.rs    yaml_parser_scan_tag(parser, token: *mut yaml_token_t) -> c_int
scanner_scan.rs    yaml_parser_scan_tag_handle(parser, directive: c_int, start_mark: yaml_mark_t, handle: *mut *mut yaml_char_t) -> c_int
scanner_scan.rs    yaml_parser_scan_tag_uri(parser, uri_char: c_int, directive: c_int, head: *mut yaml_char_t, start_mark: yaml_mark_t, uri: *mut *mut yaml_char_t) -> c_int
scanner_scan.rs    yaml_parser_scan_uri_escapes(parser, directive: c_int, start_mark: yaml_mark_t, string: *mut yaml_string_t) -> c_int
scanner_scalar.rs  yaml_parser_scan_block_scalar(parser, token: *mut yaml_token_t, literal: c_int) -> c_int
scanner_scalar.rs  yaml_parser_scan_block_scalar_breaks(parser, indent: *mut c_int, breaks: *mut yaml_string_t, start_mark: yaml_mark_t, end_mark: *mut yaml_mark_t) -> c_int
scanner_scalar.rs  yaml_parser_scan_flow_scalar(parser, token: *mut yaml_token_t, single: c_int) -> c_int
scanner_scalar.rs  yaml_parser_scan_plain_scalar(parser, token: *mut yaml_token_t) -> c_int
parser.rs       yaml_parser_parse(parser, event: *mut yaml_event_t) -> c_int                     [no_mangle extern "C"]
parser.rs       yaml_get_max_nest_level() -> c_int                                               [no_mangle extern "C"]
parser.rs       yaml_set_max_nest_level(max: c_int)                                              [no_mangle extern "C"]
parser.rs       yaml_parser_set_parser_error(parser, problem: *const c_char, problem_mark: yaml_mark_t) -> c_int   [pub(crate)]
parser.rs       yaml_parser_set_parser_error_context(parser, context: *const c_char, context_mark: yaml_mark_t, problem: *const c_char, problem_mark: yaml_mark_t) -> c_int  [pub(crate)]
parser.rs       yaml_parser_state_machine(parser, event) -> c_int                                [pub(crate)]
parser.rs       yaml_parser_parse_stream_start(parser, event) -> c_int
parser.rs       yaml_parser_parse_document_start(parser, event, implicit: c_int) -> c_int
parser.rs       yaml_parser_parse_document_content(parser, event) -> c_int
parser.rs       yaml_parser_parse_document_end(parser, event) -> c_int
parser.rs       yaml_parser_parse_node(parser, event, block: c_int, indentless_sequence: c_int) -> c_int
parser.rs       yaml_parser_parse_block_sequence_entry(parser, event, first: c_int) -> c_int
parser.rs       yaml_parser_parse_indentless_sequence_entry(parser, event) -> c_int
parser_states.rs yaml_parser_parse_block_mapping_key(parser, event, first: c_int) -> c_int
parser_states.rs yaml_parser_parse_block_mapping_value(parser, event) -> c_int
parser_states.rs yaml_parser_parse_flow_sequence_entry(parser, event, first: c_int) -> c_int
parser_states.rs yaml_parser_parse_flow_sequence_entry_mapping_key(parser, event) -> c_int
parser_states.rs yaml_parser_parse_flow_sequence_entry_mapping_value(parser, event) -> c_int
parser_states.rs yaml_parser_parse_flow_sequence_entry_mapping_end(parser, event) -> c_int
parser_states.rs yaml_parser_parse_flow_mapping_key(parser, event, first: c_int) -> c_int
parser_states.rs yaml_parser_parse_flow_mapping_value(parser, event, empty: c_int) -> c_int
parser_states.rs yaml_parser_process_empty_scalar(parser, event, mark: yaml_mark_t) -> c_int
parser_states.rs yaml_parser_process_directives(parser, version_directive_ref: *mut *mut yaml_version_directive_t, tag_directives_start_ref: *mut *mut yaml_tag_directive_t, tag_directives_end_ref: *mut *mut yaml_tag_directive_t) -> c_int
parser_states.rs yaml_parser_append_tag_directive(parser, value: yaml_tag_directive_t, allow_duplicates: c_int, mark: yaml_mark_t) -> c_int
loader.rs       yaml_parser_load(parser, document: *mut yaml_document_t) -> c_int                [no_mangle extern "C"]
dumper.rs       yaml_emitter_open(emitter) -> c_int                                              [no_mangle extern "C"]
dumper.rs       yaml_emitter_close(emitter) -> c_int                                             [no_mangle extern "C"]
dumper.rs       yaml_emitter_dump(emitter, document: *mut yaml_document_t) -> c_int              [no_mangle extern "C"]
emitter.rs      yaml_emitter_emit(emitter, event: *mut yaml_event_t) -> c_int                    [no_mangle extern "C"]
emitter.rs      yaml_emitter_set_emitter_error(emitter, problem: *const c_char) -> c_int         [pub(crate)]
emitter.rs      yaml_emitter_increase_indent(emitter, flow: c_int, indentless: c_int) -> c_int   [pub(crate)]
emitter.rs      yaml_emitter_append_tag_directive(emitter, value: yaml_tag_directive_t, allow_duplicates: c_int) -> c_int  [pub(crate)]
emitter.rs      yaml_emitter_state_machine(emitter, event) -> c_int                              [pub(crate)]
emitter.rs      yaml_emitter_need_more_events(emitter) -> c_int                                  [pub(crate)]
emitter.rs      yaml_emitter_emit_node(emitter, event, root: c_int, sequence: c_int, mapping: c_int, simple_key: c_int) -> c_int
emitter_analyze.rs yaml_emitter_check_empty_document(emitter) -> c_int
emitter_analyze.rs yaml_emitter_check_empty_sequence(emitter) -> c_int
emitter_analyze.rs yaml_emitter_check_empty_mapping(emitter) -> c_int
emitter_analyze.rs yaml_emitter_check_simple_key(emitter) -> c_int
emitter_analyze.rs yaml_emitter_select_scalar_style(emitter, event) -> c_int
emitter_analyze.rs yaml_emitter_process_anchor(emitter) -> c_int
emitter_analyze.rs yaml_emitter_process_tag(emitter) -> c_int
emitter_analyze.rs yaml_emitter_process_scalar(emitter) -> c_int
emitter_analyze.rs yaml_emitter_analyze_version_directive(emitter, version_directive: yaml_version_directive_t) -> c_int
emitter_analyze.rs yaml_emitter_analyze_tag_directive(emitter, tag_directive: yaml_tag_directive_t) -> c_int
emitter_analyze.rs yaml_emitter_analyze_anchor(emitter, anchor: *mut yaml_char_t, alias: c_int) -> c_int
emitter_analyze.rs yaml_emitter_analyze_tag(emitter, tag: *mut yaml_char_t) -> c_int
emitter_analyze.rs yaml_emitter_analyze_scalar(emitter, value: *mut yaml_char_t, length: size_t) -> c_int
emitter_analyze.rs yaml_emitter_analyze_event(emitter, event: *mut yaml_event_t) -> c_int
emitter_write.rs yaml_emitter_write_bom(emitter) -> c_int
emitter_write.rs yaml_emitter_write_indent(emitter) -> c_int
emitter_write.rs yaml_emitter_write_indicator(emitter, indicator: *const c_char, need_whitespace: c_int, is_whitespace: c_int, is_indention: c_int) -> c_int
emitter_write.rs yaml_emitter_write_anchor(emitter, value: *mut yaml_char_t, length: size_t) -> c_int
emitter_write.rs yaml_emitter_write_tag_handle(emitter, value: *mut yaml_char_t, length: size_t) -> c_int
emitter_write.rs yaml_emitter_write_tag_content(emitter, value: *mut yaml_char_t, length: size_t, need_whitespace: c_int) -> c_int
emitter_write.rs yaml_emitter_write_plain_scalar(emitter, value: *mut yaml_char_t, length: size_t, allow_breaks: c_int) -> c_int
emitter_write.rs yaml_emitter_write_single_quoted_scalar(emitter, value: *mut yaml_char_t, length: size_t, allow_breaks: c_int) -> c_int
emitter_write.rs yaml_emitter_write_double_quoted_scalar(emitter, value: *mut yaml_char_t, length: size_t, allow_breaks: c_int) -> c_int
emitter_write.rs yaml_emitter_write_block_scalar_hints(emitter, string: yaml_string_t) -> c_int
emitter_write.rs yaml_emitter_write_literal_scalar(emitter, value: *mut yaml_char_t, length: size_t) -> c_int
emitter_write.rs yaml_emitter_write_folded_scalar(emitter, value: *mut yaml_char_t, length: size_t) -> c_int
api.rs          yaml_parser_initialize / yaml_parser_delete / yaml_parser_set_input_string /
                yaml_parser_set_input_file / yaml_parser_set_input / yaml_parser_set_encoding /
                yaml_emitter_initialize / yaml_emitter_delete / yaml_emitter_set_output_string /
                yaml_emitter_set_output_file / yaml_emitter_set_output / yaml_emitter_set_encoding /
                yaml_emitter_set_canonical / yaml_emitter_set_indent / yaml_emitter_set_width /
                yaml_emitter_set_unicode / yaml_emitter_set_break      [all no_mangle extern "C"]
api_objects.rs  yaml_token_delete / yaml_*_event_initialize / yaml_event_delete  [no_mangle extern "C"]
                yaml_check_utf8(start: *const yaml_char_t, length: size_t) -> c_int   [pub(crate)]
api_document.rs yaml_document_initialize / yaml_document_delete / yaml_document_get_node /
                yaml_document_get_root_node / yaml_document_add_scalar /
                yaml_document_add_sequence / yaml_document_add_mapping /
                yaml_document_append_sequence_item / yaml_document_append_mapping_pair
                [all no_mangle extern "C"]
```

## Your final report must contain

1. The exact file(s) you created.
2. Any function in your range you could NOT translate (there should be none).
3. Any pitfall you hit or any mismatch you noticed with this contract
   (e.g. a macro that doesn't fit, a helper you needed that is missing).
4. Any symbol you referenced that belongs to another module, with the exact
   signature you assumed.

Do NOT run `cargo build` yourself unless your module is self-contained — the
other modules are being written in parallel and the crate will not compile until
all of them land. `cargo check` output full of "cannot find function in module
X" is expected and is not your problem. You MAY run
`rustc --edition 2021 --crate-type lib -o /dev/null <your file>` style syntax
checks only if useful; otherwise just be careful.
