# `#[zyn::derive]`

Replaces `#[proc_macro_derive(...)]`. The annotated item is parsed as `syn::DeriveInput` and wrapped in `Input::Derive`. All parameters must be `#[zyn(input)]` extractors — derive macros don't receive arguments.

## Basic Derive

The derive name is the PascalCase of the function name:

```rust,zyn
#[zyn::derive]
fn my_builder(
    #[zyn(input)] ident: zyn::Extract<zyn::syn::Ident>,
) -> zyn::TokenStream {
    zyn::zyn!(
        impl {{ ident }} {
            pub fn build(self) -> Self { self }
        }
    )
}
```

The function name `my_builder` becomes `MyBuilder`. Users apply it to a struct:

```rust
#[derive(MyBuilder)]
struct Config {
    host: String,
    port: u16,
}

// Generated:
// impl Config {
//     pub fn build(self) -> Self { self }
// }
```

## Multiple Extractors

Extract as many input properties as you need:

```rust,zyn
#[zyn::derive]
fn my_builder(
    #[zyn(input)] ident: zyn::Extract<zyn::syn::Ident>,
    #[zyn(input)] fields: zyn::Fields,
    #[zyn(input)] generics: zyn::Extract<zyn::syn::Generics>,
) -> zyn::TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    zyn::zyn!(
        impl {{ impl_generics }} {{ ident }} {{ ty_generics }} {{ where_clause }} {
            @for (field in fields.iter()) {
                pub fn {{ field.ident.as_ref().unwrap() | snake }}(
                    mut self,
                    value: {{ field.ty.clone() }},
                ) -> Self {
                    self.{{ field.ident.as_ref().unwrap() }} = value;
                    self
                }
            }
        }
    )
}
```

Usage with generics:

```rust
#[derive(MyBuilder)]
struct Config<T: Default> {
    host: String,
    value: T,
}

// Generated:
// impl<T: Default> Config<T> {
//     pub fn host(mut self, value: String) -> Self { self.host = value; self }
//     pub fn value(mut self, value: T) -> Self { self.value = value; self }
// }
```

## Custom Name

Override the derive name with a string argument:

```rust,zyn
#[zyn::derive("Builder")]
fn internal_builder_impl(
    #[zyn(input)] ident: zyn::Extract<zyn::syn::Ident>,
    #[zyn(input)] fields: zyn::Fields,
) -> zyn::TokenStream {
    zyn::zyn!(
        impl {{ ident }} {
            @for (field in fields.iter()) {
                pub fn {{ field.ident.as_ref().unwrap() | snake }}(
                    mut self,
                    value: {{ field.ty.clone() }},
                ) -> Self {
                    self.{{ field.ident.as_ref().unwrap() }} = value;
                    self
                }
            }
        }
    )
}
```

Users write `#[derive(Builder)]` — the function name doesn't matter:

```rust
#[derive(Builder)]
struct Config {
    host: String,
    port: u16,
}
```

## Helper Attributes

Declare helper attributes so users can annotate fields:

```rust,zyn
#[zyn::derive("Builder", attributes(builder))]
fn builder_derive(
    #[zyn(input)] ident: zyn::Extract<zyn::syn::Ident>,
    #[zyn(input)] fields: zyn::Fields,
) -> zyn::TokenStream {
    zyn::zyn!(
        impl {{ ident }} {
            @for (field in fields.iter()) {
                pub fn {{ field.ident.as_ref().unwrap() | snake }}(
                    mut self,
                    value: {{ field.ty.clone() }},
                ) -> Self {
                    self.{{ field.ident.as_ref().unwrap() }} = value;
                    self
                }
            }
        }
    )
}
```

Users can now write:

```rust
#[derive(Builder)]
struct Config {
    #[builder(default = "8080")]
    port: u16,
    host: String,
}
```

Multiple helpers are comma-separated: `attributes(builder, validate)`.

## With Attribute Parsing

Combine helper attributes with `#[derive(Attribute)]` for typed extraction:

```rust,zyn
#[derive(zyn::Attribute)]
#[zyn("builder")]
struct BuilderConfig {
    #[zyn(default)]
    skip: bool,
}

#[zyn::derive("Builder", attributes(builder))]
fn builder_derive(
    #[zyn(input)] ident: zyn::Extract<zyn::syn::Ident>,
    #[zyn(input)] fields: zyn::Fields,
    #[zyn(input)] cfg: zyn::Attr<BuilderConfig>,
) -> zyn::TokenStream {
    if cfg.skip {
        return zyn::zyn!();
    }
    zyn::zyn!(
        impl {{ ident }} {
            @for (field in fields.iter()) {
                pub fn {{ field.ident.as_ref().unwrap() | snake }}(
                    mut self,
                    value: {{ field.ty.clone() }},
                ) -> Self {
                    self.{{ field.ident.as_ref().unwrap() }} = value;
                    self
                }
            }
        }
    )
}
```

Usage:

```rust
#[derive(Builder)]
struct Config {
    #[builder(skip)]
    internal: bool,
    host: String,
}
```

## Using Elements

Elements work inside derive bodies because `input` is in scope:

```rust,zyn
#[zyn::element]
fn setter(name: zyn::syn::Ident, ty: zyn::syn::Type) -> zyn::TokenStream {
    zyn::zyn!(
        pub fn {{ name | snake }}(mut self, value: {{ ty }}) -> Self {
            self.{{ name }} = value;
            self
        }
    )
}

#[zyn::derive]
fn my_builder(
    #[zyn(input)] ident: zyn::Extract<zyn::syn::Ident>,
    #[zyn(input)] fields: zyn::Fields,
) -> zyn::TokenStream {
    zyn::zyn!(
        impl {{ ident }} {
            @for (field in fields.iter()) {
                @setter(
                    name = field.ident.clone().unwrap(),
                    ty = field.ty.clone(),
                )
            }
        }
    )
}
```

Usage is identical — the element is an implementation detail:

```rust
#[derive(MyBuilder)]
struct User {
    name: String,
    age: u32,
}

// Generated:
// impl User {
//     pub fn name(mut self, value: String) -> Self { self.name = value; self }
//     pub fn age(mut self, value: u32) -> Self { self.age = value; self }
// }
```

## Diagnostics

All diagnostic macros are available — `error!`, `warn!`, `note!`, `help!`, `bail!`:

```rust,zyn
#[zyn::derive]
fn my_derive(
    #[zyn(input)] fields: zyn::Fields,
    #[zyn(input)] ident: zyn::Extract<zyn::syn::Ident>,
) -> zyn::TokenStream {
    if fields.is_empty() {
        bail!("at least one field is required");
    }

    for field in fields.iter() {
        if field.ident.is_none() {
            error!("tuple structs are not supported"; span = field.span());
        }
    }
    bail!();

    warn!("this derive is experimental");

    zyn::zyn!(impl {{ ident }} {})
}
```

`bail!()` with no arguments returns early only if errors have been pushed. `bail!("msg")` pushes an error and returns immediately. See [Diagnostics](../03-elements/diagnostics.md) for full details.
