# Proc Macro Entry Points

zyn provides attribute macros that replace `#[proc_macro_derive]` and `#[proc_macro_attribute]` with a declarative, template-first workflow.

- [`#[zyn::derive]`](./derive.md) — replaces `#[proc_macro_derive]`
- [`#[zyn::attribute]`](./attribute.md) — replaces `#[proc_macro_attribute]`

## What's Generated

Both macros generate a complete proc macro entry point:

1. **Input parsing** — `DeriveInput` for derives, `Item` for attributes, wrapped in `zyn::Input`
2. **`input` binding** — available in scope for `zyn!` and element extractors
3. **Extractor resolution** — `#[zyn(input)]` parameters are resolved via `FromInput`
4. **Diagnostic macros** — `error!`, `warn!`, `note!`, `help!`, `bail!` (same as `#[zyn::element]`)
5. **Return type conversion** — your `proc_macro2::TokenStream` is automatically converted to `proc_macro::TokenStream`

You write a function that returns `zyn::TokenStream`. The macro handles everything else.
