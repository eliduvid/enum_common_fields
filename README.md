# Enum common fields
## Overview
Macro to generate getters/setters for common fields in an enum.
Meaning, that if every variant of your enum has some field,
you could access it though field() and field_mut() accessors respectively.

For example if you have common field `key` of type String you'll use the macro like this:
```rust
#[derive(EnumCommonFields)]
#[common_field(mut key: String)]
enum MyEnum {
    // Some variants
}
```
and will have methods `.key()` and `.key_mut()` that return `&String` and `&mut String` respectively.
If you don't want to create `.key_mut()` accessor, you can omit mut in the declaration like this:
```rust
#[derive(EnumCommonFields)]
#[common_field(key: String)]
enum MyEnum {
    // Some variants
}
```
## Background
It's pretty common for enum variants to have common fields, like this:
```rust
struct CreateRequest {
    id: String,
    // other fields
}
struct UpdateRequest {
    id: String,
    // other fields
}
enum Request {
    Create(CreateRequest),
    Update(UpdateRequest),
}
```
But if you have an instance of the enum, accessing the common field is pretty annoying:
```rust
fn handle(req: Request) {
    let id = match req { 
        Update(r) => r.id,
        Create(r) => r.id,
    };
    // ...
}
```
Common advice in these cases is to extract common fields into enclosing struct:
```rust
struct CreateRequest {
    // fields
}
struct UpdateRequest {
    // fields
}
enum RequestKind {
    Create(CreateRequest),
    Update(UpdateRequest),
}
struct Request {
    id: String,
    req: RequestKind,
}
```
But it complicates specific handling functions that need to use common fields. For example:

```rust
fn create_handler(
    id: String, // need to get id as a param by itself 
    req: CreateRequest
) {
    // ...
}
```
This macro solves it by auto-generating accessor methods for common fields. For example:
```rust
struct CreateRequest {
    id: String,
    // other fields
}
struct UpdateRequest {
    id: String,
    // other fields
}

#[derive(EnumCommonFields)]
#[common_field(id: String)]
enum Request {
    Create(CreateRequest),
    Update(UpdateRequest),
}
fn handle(req: Request) {
    let id = req.id();
    // ...
}
```
Generated accessor is equivalent to this one:
```rust
impl Request {
    fn id(&self) -> &String {
        match self {
            Update(r) => r.id,
            Create(r) => r.id,
        }
    }
}
```
## Installation
Add following to your `Cargo.toml`:
```toml
enum_common_fields = "0.2.0" # Use latest
```
## Usage
### Basic Usage
Add `#[derive(EnumCommonFields)]` above your enum and `#[common_field(field_name: Type)]` after it for every common field you need to generate accessors for:
```rust
use enum_common_fields::EnumCommonFields;

#[derive(EnumCommonFields)]
#[common_field(key: String)]
enum MyEnum {
    VariantOne { key: String },
    VariantTwo { key: String },
}
fn main () {
    let my_enum = MyEnum::VariantOne { key: "Example".into() };
    assert_eq!(my_enum.key(), "Example");
}
```
You can add `mut` to `common_field` annotation to also generate `<field_name>_mut()` accessor that returns mutable reference and `own` to also add `into_<field_name>()` accessor that consumes original instance:
```rust
use enum_common_fields::EnumCommonFields;

#[derive(EnumCommonFields)]
#[common_field(own key: String)] // Generates read-only, mutable and owning accessors
enum MyEnum {
    VariantOne { key: String },
    VariantTwo { key: String },
}

fn main() {
    let mut my_enum = MyEnum::VariantOne { key: "Example".into() };
    assert_eq!(my_enum.key(), "Example");

    my_enum.key_mut().push_str(" Mutated"); // Mutable access
    assert_eq!(my_enum.key(), "Example Mutated");
    
    let key: String = my_enum.into_key(); // Consuming MyEnum instance, and getting owned String instance
    assert_eq!(key, "Example Mutated".to_string())
}
```
### Modifiers
`common_field` annotation without access modifier generates only immutable accessor.
`mut_only` generates only mutable one, and `own_only` only owning one.
`mut` generates both mutable and immutable accessors, and `own` (and it's alias `all`) generate both of those and also th owning one.
If you need only mutable and owning accessor, or only immutable and owning you'll need to add more than one accessor per field:
```rust
#[derive(EnumCommonFields)]
#[common_field(key: String)] // Generate only immutable accessor
#[common_field(own_only key: String)] // And only owning accessor
enum MyEnum {
    VariantOne { key: String },
    VariantTwo { key: String },
}
```
### Renaming
You can use `as getter_name` in the `common_field` annotation to rename generated function name. You can use `as` only in `common_field` annotations with modifiers that generate only one accessor (`own_only`/`mut_only`/no modifier). If you need to rename more than one accessor for one field you once more will need to add more than one annotation per field:
```rust
#[derive(EnumCommonFields)]
#[common_field(key as k: String)]
#[common_field(mut_only key as k_mut: String)]
#[common_field(own_only key as into_k: String)]
enum MyEnum {
    VariantOne { key: String },
    VariantTwo { key: String },
}
fn main() {
    let mut my_enum = MyEnum::VariantOne { key: "Example".into() };
    assert_eq!(my_enum.k(), "Example");

    my_enum.k_mut().push_str(" Mutated"); // Mutable access
    assert_eq!(my_enum.k(), "Example Mutated");

    let key: String = my_enum.into_k(); // Consuming MyEnum instance, and getting owned String instance
    assert_eq!(key, "Example Mutated".to_string())
}
```
If you want, you can generate multiple accessors with different names for the same field:
```rust
#[derive(EnumCommonFields)]
#[common_field(key: String)] // Generates accessor named key()
#[common_field(key as k: String)] // Generates accessor named k()
#[common_field(key as get_key: String)] // Generates accessor named get_key()
enum MyEnum {
    VariantOne { key: String },
    VariantTwo { key: String },
}
```
## Limitations
Does not support struct enums.
```rust
#[derive(EnumCommonFields)]
#[common_field(key: String)] // Will not compile: "expected tuple struct or tuple variant, found struct variant `Self::VariantOne`"
enum TestEnum {
    VariantOne { key: String },
    VariantTwo { key: String },
}
```
May add support for those in the future. Tuple enums support is not planned.
## Missing features
There are some features that are implementable, but I'm not convinced that effort of adding them is worth it. So if you are one of the lucky few that has a real use-case for one of those, feel free to pester me in the issues.
### Bulk-renaming accessors
I'm talking both renaming 'base' field name for accessors (so for field `identifier` would be generated `id()` and `id_mut()`) and changing accessor name 'template' (so all immutable accessors will be `get_field()` instead of just `field()`). It's possible, but I'm sure that most people will be totally OK with my convention.
### Composite owning accessors
As of now, owning accessors are pretty limited. If you want to take ownership of more than one common field of the enum instance, you need to fall back to using `match` with many identical branches. Theoretically I could generate some `into_common()` method, that will return all fields that have owning accessors. The problem is return type of this theoretical method. Generating struct for all the common fields seems like too much, but just returning a tuple may be very confusing if many fields have same type.
### Weird combinations of accessors with one annotation
I just don't believe that somebody needs to generate only owning and mutable accessor for a field frequently enough to talk about it.
### Conversions
As of now, only conversion that the macro performs are those from `Deref` and `DerefMut` traits. For example, you can use `str` as a type of read only accessor of `String` field. This way the accessor will return `&str`. But it does not call `into()` or any other conversions.
