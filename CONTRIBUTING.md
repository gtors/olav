# Contributing to olav

Thanks for your interest in contributing! This is a small project — most
contributions fall into one of these buckets.

## Setup

The crate is a workspace with two members. A stable Rust toolchain is enough.

```sh
cargo build
cargo test           # all unit, integration, doc, and trybuild tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

A clean `cargo test` plus clippy and fmt all green is the bar.

## Tests

| Suite | What it covers |
|---|---|
| `crates/olav/tests/basic.rs` | bare elements, text, attrs, control flow basics |
| `crates/olav/tests/control_flow.rs` | `@if`, `@for`, `@while`, `@match`; Option/Vec/array splices |
| `crates/olav/tests/namespaces.rs` | bracketed element/attr names, self-closing, nesting |
| `crates/olav/tests/special_nodes.rs` | `?xml`, `?xml-stylesheet`, `!DOCTYPE` (with internal subset), `@cdata`, `@comment` |
| `crates/olav/tests/roundtrip.rs` | parses generated XML with `quick-xml` to verify well-formedness |
| `crates/olav/tests/atom_feed.rs` | the README Atom-feed example, enforced as a test |
| `crates/olav/tests/extras.rs` | Cyrillic, `standalone`, names with digits/dashes |
| `crates/olav/tests/trybuild.rs` | compile-error UX (snapshots in `tests/ui/*.stderr`) |
| doc tests | rustdoc examples on every public item |

When changing the macro behaviour, also update the README example that
matches — both the Atom feed (`tests/atom_feed.rs`) and the Cyrillic
example (`tests/extras.rs::cyrillic_full_document_example_from_readme`) are
enforced as tests.

## Updating trybuild snapshots

```sh
TRYBUILD=overwrite cargo test --test trybuild
```

Then commit the new `.stderr` files.

## Opening a PR

- Run `cargo test` plus clippy and fmt before pushing.
- If your change adds a public API, add a rustdoc example.
- If your change alters the macro syntax, update `README.md` and add a fixture in `tests/ui/`.

## Architecture quick-reference

When changing the macro behaviour, read these in order:

1. `crates/olav_macros/src/ast.rs` — the AST types
2. `crates/olav_macros/src/parse.rs` — token-stream → AST
3. `crates/olav_macros/src/codegen.rs` — AST → `quote!` output
4. `crates/olav/src/{markup,render,escape,pre_escaped}.rs` — runtime

The macro always emits code of the shape:

```rust
{
    let mut __olav_buf = ::olav::Markup::new();
    let __olav_out = &mut __olav_buf;
    /* generated statements */
    __olav_buf
}
```

Generated variable names start with `__olav_` — if you ever change that prefix,
update `parse.rs`, `codegen.rs`, and the CDATA substitution regexes together.

## License

By submitting a contribution you agree it will be MIT-licensed (matching the
project license; see `LICENSE-MIT`).
