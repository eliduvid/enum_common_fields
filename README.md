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
enum_common_fields = "0.5.0"
```
## Usage
See [the docs](https://docs.rs/enum_common_fields/latest/enum_common_fields/derive.EnumCommonFields.html) for a complete reference.
## Missing features
There are some features that are implementable, but I'm not convinced that effort of adding them is worth it. So if you are one of the lucky few that has a real use-case for one of those, feel free to pester me in the issues.
### Bulk-renaming accessors
I'm talking both renaming 'base' field name for accessors (so for field `identifier` would be generated `id()` and `id_mut()`) and changing accessor name 'template' (so all immutable accessors will be `get_field()` instead of just `field()`). It's possible, but I'm sure that most people will be totally OK with my convention.
### Composite owning accessors
As of now, owning accessors are pretty limited. If you want to take ownership of more than one common field of the enum instance, you need to fall back to using `match` with many identical branches. Theoretically I could generate some `into_common()` method, that will return all fields that have owning accessors. The problem is return type of this theoretical method. Generating struct for all the common fields seems like too much, but just returning a tuple may be very confusing if many fields have same type.
### Weird combinations of accessors with one annotation
I just don't believe that somebody needs to generate only owning and mutable accessor for a field frequently enough to talk about it.
### Conversions
As of now, only conversion that the macro performs are those from `Deref` and `DerefMut` traits. For example, you can use `str` as a type of ref accessors of `String` field. This way the accessors will return `&str` and `&mut str`. But it does not call `into()` or any other conversions.
