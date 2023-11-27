extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::parse::discouraged::Speculative;
use syn::parse::ParseStream;
use syn::{parse_macro_input, DataEnum, DeriveInput, Fields, Meta, Token};

#[derive(Clone, Eq, PartialEq, Debug)]
enum GetterKind {
    ReadOnly,
    Mutable,
    Owning,
}

impl GetterKind {
    fn parse(input: ParseStream) -> syn::Result<Vec<Self>> {
        if input.peek(syn::Ident) && input.peek2(Token![:]) {
            return Ok(vec![Self::ReadOnly]);
        }
        let fork = input.fork();
        if fork.parse::<Token![mut]>().is_ok() {
            input.advance_to(&fork);
            return Ok(vec![Self::ReadOnly, Self::Mutable]);
        }

        if let Ok(indent) = fork.parse::<Ident>() {
            match indent.to_string().as_str() {
                "mut_only" => {
                    input.advance_to(&fork);
                    return Ok(vec![Self::Mutable]);
                }
                "all" | "own" => {
                    input.advance_to(&fork);
                    return Ok(vec![Self::Owning, Self::Mutable, Self::ReadOnly]);
                }
                "own_only" => {
                    input.advance_to(&fork);
                    return Ok(vec![Self::Owning]);
                }
                _ => {}
            }
        }

        Ok(vec![Self::ReadOnly])
    }
}

/// Internal struct to store parameters for EnumCommonFields
#[derive(Clone)]
struct CommonField {
    kinds: Vec<GetterKind>,
    field_name: Ident,
    field_type: Ident,
    resulting_name: Option<Ident>, // Can have a value only if one function is generated
}

impl syn::parse::Parse for CommonField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kinds = GetterKind::parse(input)?;
        let field_name = input.parse()?;
        let resulting_name = match input.parse::<Token![as]>() {
            Ok(_) => Some(input.parse::<Ident>()?),
            Err(_) => None,
        };
        input.parse::<Token![:]>()?;
        let field_type = input.parse()?;
        Ok(Self {
            kinds,
            field_name,
            field_type,
            resulting_name,
        })
    }
}

#[derive(Clone)]
struct EnumVariantInfo {
    name: Ident,
    is_struct: bool,
}

fn parse_enum_variants(enum_info: DataEnum) -> Vec<EnumVariantInfo> {
    enum_info
        .variants
        .into_iter()
        .map(|variant| EnumVariantInfo {
            is_struct: match variant.fields {
                Fields::Named(_) => true,
                Fields::Unnamed(_) => false,
                Fields::Unit => panic!(
                    "Variant {} is a unit variant, which is not supported",
                    variant.ident
                ),
            },
            name: variant.ident,
        })
        .collect()
}

/// Macro to generate getters/setters for common fields in an enum.
/// Meaning, that if every variant of your enum has some field,
/// you could access it though field(), field_mut() or into_field() accessors respectively.
///
/// For example if you have common field `key` of type String you'll use the macro like this:
/// ```
/// use enum_common_fields::EnumCommonFields;
///
/// #[derive(EnumCommonFields)]
/// #[common_field(mut key: String)]
/// enum MyEnum {
/// #   _V{ key: String }
///     // Some variants
/// }
/// ```
/// and will have methods `.key()` and `.key_mut()` that return `&String` and `&mut String` respectively.
/// If you don't want to create `.key_mut()` accessor, you can omit mut in the declaration like this:
/// ```
/// # use enum_common_fields::EnumCommonFields;
/// #[derive(EnumCommonFields)]
/// #[common_field(key: String)]
/// enum MyEnum {
/// #   _V{ key: String }
///     // Some variants
/// }
/// ```
/// Generated accessor functions contain only match statement on current enum instance
/// and extraction of the field in each branch.
///
/// ### Basic Usage
/// Add `#[derive(EnumCommonFields)]` above your enum and `#[common_field(field_name: Type)]` after it for every common field you need to generate accessors for:
/// ```rust
/// # use enum_common_fields::EnumCommonFields;
/// struct VariantOne {
///     key: String
/// }
///
/// #[derive(EnumCommonFields)]
/// #[common_field(key: String)]
/// enum MyEnum {
///     VariantOne(VariantOne),
///     VariantTwo {
///         key: String
///     },
/// }
/// let my_enum = MyEnum::VariantOne (VariantOne { key: "Example".into() });
/// assert_eq!(my_enum.key(), "Example");
/// ```
/// You can add `mut` to `common_field` annotation to also generate `<field_name>_mut()` accessor that returns mutable reference and `own` to also add `into_<field_name>()` accessor that consumes original instance:
/// ```rust
/// # use enum_common_fields::EnumCommonFields;
///
/// struct VariantTwo {
///     key: String
/// }
///
/// #[derive(EnumCommonFields)]
/// #[common_field(own key: String)] // Generates read-only, mutable and owning accessors
/// enum MyEnum {
///     VariantOne {
///         key: String
///     },
///     VariantTwo(VariantTwo),
/// }
///
/// let mut my_enum = MyEnum::VariantOne { key: "Example".into() };
/// assert_eq!(my_enum.key(), "Example");
///
/// my_enum.key_mut().push_str(" Mutated"); // Mutable access
/// assert_eq!(my_enum.key(), "Example Mutated");
///
/// let key: String = my_enum.into_key(); // Consuming MyEnum instance, and getting owned String instance
/// assert_eq!(key, "Example Mutated".to_string())
/// ```
/// As you can see, both struct variants and tuple variants with a single struct are supported.
/// Enums with unit variants or multiple things in a tuple variant are not.
/// ### Modifiers
/// `common_field` annotation without access modifier generates only immutable accessor.
/// `mut_only` generates only mutable one, and `own_only` only owning one.
/// `mut` generates both mutable and immutable accessors, and `own` (and it's alias `all`) generate both of those and also the owning one.
/// If you need only mutable and owning accessor, or only immutable and owning you'll need to add more than one accessor per field:
/// ```rust
/// # use enum_common_fields::EnumCommonFields;
/// struct VariantOne {
///     key: String
/// }
///
/// struct VariantTwo {
///     key: String
/// }
///
/// #[derive(EnumCommonFields)]
/// #[common_field(key: String)] // Generate only immutable accessor
/// #[common_field(own_only key: String)] // And only owning accessor
/// enum MyEnum {
///     VariantOne(VariantOne),
///     VariantTwo(VariantTwo),
/// }
/// ```
/// ### Types
/// Type in the `#[common_field]` annotation is used only as a return type of the accessor.
/// So you if you generate only reference accessors (or you generate owning accessor in a different annotation)
/// you can use type that `Deref`s from the original field type instead of it itself.
/// Classic example is using `str` instead of `String` for reference accessors:
/// ```rust
/// # use enum_common_fields::EnumCommonFields;
/// #[derive(EnumCommonFields)]
/// #[common_field(mut key: str)]
/// #[common_field(own_only key: String)]
/// enum MyEnum {
///     One { key: String }
/// }
/// let mut e = MyEnum::One { key: "k".to_string() };
/// let key_ref = e.key(); // returns "k" as &str instead or &String
/// let key_mut_ref = e.key_mut(); // returns "k" as &mut str instead or &mut String
/// let key = e.into_key(); // consumes e and returns "k" as actual String
/// ```
/// ### Renaming
/// You can use `as getter_name` in the `common_field` annotation to rename generated function name. You can use `as` only in `common_field` annotations with modifiers that generate only one accessor (`own_only`/`mut_only`/no modifier). If you need to rename more than one accessor for one field you once more will need to add more than one annotation per field:
/// ```rust
/// # use enum_common_fields::EnumCommonFields;
/// struct VariantOne {
///     key: String
/// }
///
/// struct VariantTwo {
///     key: String
/// }
///
/// #[derive(EnumCommonFields)]
/// #[common_field(key as k: String)]
/// #[common_field(mut_only key as k_mut: String)]
/// #[common_field(own_only key as into_k: String)]
/// enum MyEnum {
///     VariantOne(VariantOne),
///     VariantTwo(VariantTwo),
/// }
///
/// let mut my_enum = MyEnum::VariantOne(VariantOne { key: "Example".into() });
/// assert_eq!(my_enum.k(), "Example");
///
/// my_enum.k_mut().push_str(" Mutated"); // Mutable access
/// assert_eq!(my_enum.k(), "Example Mutated");
///
/// let key: String = my_enum.into_k(); // Consuming MyEnum instance, and getting owned String instance
/// assert_eq!(key, "Example Mutated".to_string())
/// ```
/// If you want, you can generate multiple accessors with different names for the same field:
/// ```rust
/// # use enum_common_fields::EnumCommonFields;
/// #[derive(EnumCommonFields)]
/// #[common_field(key: String)] // Generates accessor named key()
/// #[common_field(key as k: String)] // Generates accessor named k()
/// #[common_field(key as get_key: String)] // Generates accessor named get_key()
/// enum MyEnum {
///     VariantOne { key: String, /* other fields */ },
///     VariantTwo { key: String, /* other fields */ },
/// }
/// ```
#[proc_macro_derive(EnumCommonFields, attributes(common_field))]
pub fn common_fields_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let common_fields = parse_common_fields_attributes(&ast);

    if common_fields.is_empty() {
        panic!("EnumCommonFields requires at least one #[common_field] annotation")
    }

    let enum_name = ast.ident;
    let variants: Vec<_> = match ast.data {
        syn::Data::Enum(e) => parse_enum_variants(e),
        _ => panic!("EnumCommonFields can only be applied to enums"),
    };

    if variants.is_empty() {
        return TokenStream::new();
    }

    let mut stream = quote!();

    for CommonField {
        kinds,
        field_name,
        field_type,
        resulting_name,
    } in common_fields
    {
        if resulting_name.is_some() && kinds.len() != 1 {
            panic!("\"as getter_name\" syntax is supported only for single getter annotations (own_only, mut_only or immutable [no annotations])")
        }
        for kind in kinds {
            match kind {
                GetterKind::ReadOnly => {
                    stream.extend(generate_accessor(
                        &enum_name,
                        &variants,
                        &field_name,
                        &field_type,
                        quote!(&),
                        resulting_name.clone().unwrap_or_else(|| field_name.clone()),
                    ));
                }
                GetterKind::Mutable => {
                    stream.extend(generate_accessor(
                        &enum_name,
                        &variants,
                        &field_name,
                        &field_type,
                        quote!(&mut),
                        resulting_name
                            .clone()
                            .unwrap_or_else(|| format_ident!("{field_name}_mut")),
                    ));
                }
                GetterKind::Owning => {
                    stream.extend(generate_accessor(
                        &enum_name,
                        &variants,
                        &field_name,
                        &field_type,
                        quote!(),
                        resulting_name
                            .clone()
                            .unwrap_or_else(|| format_ident!("into_{field_name}")),
                    ));
                }
            }
        }
    }
    TokenStream::from(stream)
}

fn generate_accessor(
    enum_name: &Ident,
    variants: &Vec<EnumVariantInfo>,
    field_name: &Ident,
    field_type: &Ident,
    ref_token: proc_macro2::TokenStream,
    resulting_name: Ident,
) -> proc_macro2::TokenStream {
    let match_branches: Vec<_> = variants
        .clone()
        .iter()
        .map(|EnumVariantInfo { name, is_struct }| {
            if *is_struct {
                quote!(Self::#name{#field_name, ..} => #field_name)
            } else {
                quote!(Self::#name(v) => #ref_token v.#field_name)
            }
        })
        .collect();
    quote! {
        impl #enum_name {
            pub fn #resulting_name(#ref_token self) -> #ref_token #field_type {
                match self {
                    #(#match_branches,)*
                }
            }
        }
    }
}

fn parse_common_fields_attributes(ast: &DeriveInput) -> Vec<CommonField> {
    ast
        .attrs
        .iter()
        .filter_map(|attr| {
            // Checking that we have only #[common_field ...] attributes
            if attr.path().is_ident("common_field") {
                // Checking that the attribute has parenthesis like this #[common_field(...)]
                if let Meta::List(list) = &attr.meta {
                    // Parsing data of the attribute
                    Some(syn::parse2::<CommonField>(list.tokens.clone()).unwrap())
                } else {
                    panic!("Expected format: #[common_field([all|own|own_only|mut|mut_only] field_name [as getter_name]: Type)]")
                }
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod common_field_parsing_tests {
    use super::*;
    use syn::parse_quote;
    #[test]
    fn test_basic_field() {
        let tokens = parse_quote! { field1: i32 };
        let parsed: CommonField = syn::parse2(tokens).expect("Failed to parse");

        assert_eq!(parsed.field_name, "field1");
        assert_eq!(parsed.field_type, "i32");
        assert_eq!(parsed.kinds, vec![GetterKind::ReadOnly]);
        assert!(parsed.resulting_name.is_none());
    }

    #[test]
    fn test_field_with_custom_name() {
        let tokens = parse_quote! { field1 as custom_name: i32 };
        let parsed: CommonField = syn::parse2(tokens).expect("Failed to parse");

        assert_eq!(parsed.field_name, "field1");
        assert_eq!(parsed.resulting_name.unwrap(), "custom_name");
        assert_eq!(parsed.field_type, "i32");
        assert_eq!(parsed.kinds, vec![GetterKind::ReadOnly]);
    }

    #[test]
    fn test_mutable_field() {
        let tokens = parse_quote! { mut field1: i32 };
        let parsed: CommonField = syn::parse2(tokens).expect("Failed to parse");

        assert_eq!(parsed.field_name, "field1");
        assert_eq!(parsed.field_type, "i32");
        assert_eq!(
            parsed.kinds,
            vec![GetterKind::ReadOnly, GetterKind::Mutable]
        );
        assert!(parsed.resulting_name.is_none());
    }

    #[test]
    fn test_owning_field() {
        let tokens = parse_quote! { own_only field1: i32 };
        let parsed: CommonField = syn::parse2(tokens).expect("Failed to parse");

        assert_eq!(parsed.field_name, "field1");
        assert_eq!(parsed.field_type, "i32");
        assert_eq!(parsed.kinds, vec![GetterKind::Owning]);
        assert!(parsed.resulting_name.is_none());
    }

    #[test]
    fn test_all_field() {
        let tokens = parse_quote! { all field1: i32 };
        let parsed: CommonField = syn::parse2(tokens).expect("Failed to parse");

        assert_eq!(parsed.field_name, "field1");
        assert_eq!(parsed.field_type, "i32");
        assert_eq!(
            parsed.kinds,
            vec![
                GetterKind::Owning,
                GetterKind::Mutable,
                GetterKind::ReadOnly
            ]
        );
        assert!(parsed.resulting_name.is_none());
    }

    #[test]
    fn test_invalid_format() {
        let tokens = parse_quote! { field1 i32 };
        let result: Result<CommonField, _> = syn::parse2(tokens);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod attributes_parse_tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_no_common_field() {
        let input: DeriveInput = parse_quote! {
            #[derive(Debug)]
            enum TestEnum {
                Variant1 { field1: i32 },
                Variant2 { field1: i32 },
            }
        };

        let result = parse_common_fields_attributes(&input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_common_field() {
        let input: DeriveInput = parse_quote! {
            #[derive(Debug)]
            #[common_field(field1: i32)]
            enum TestEnum {
                Variant1 { field1: i32 },
                Variant2 { field1: i32 },
            }
        };

        let result = parse_common_fields_attributes(&input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].field_name, "field1");
        assert_eq!(result[0].field_type, "i32");
        assert_eq!(result[0].kinds, vec![GetterKind::ReadOnly]);
    }

    #[test]
    fn test_multiple_common_fields() {
        let input: DeriveInput = parse_quote! {
            #[derive(Debug)]
            #[common_field(field1: i32)]
            #[common_field(mut field2: String)]
            enum TestEnum {
                Variant1 { field1: i32, field2: String },
                Variant2 { field1: i32, field2: String },
            }
        };

        let result = parse_common_fields_attributes(&input);
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].field_name, "field1");
        assert_eq!(result[0].field_type, "i32");
        assert_eq!(result[0].kinds, vec![GetterKind::ReadOnly]);

        assert_eq!(result[1].field_name, "field2");
        assert_eq!(result[1].field_type, "String");
        assert_eq!(
            result[1].kinds,
            vec![GetterKind::ReadOnly, GetterKind::Mutable]
        );
    }

    #[test]
    fn test_common_field_with_custom_name() {
        let input: DeriveInput = parse_quote! {
            #[derive(Debug)]
            #[common_field(field1 as custom_name: i32)]
            enum TestEnum {
                Variant1 { field1: i32 },
                Variant2 { field1: i32 },
            }
        };

        let result = parse_common_fields_attributes(&input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].field_name, "field1");
        assert_eq!(result[0].clone().resulting_name.unwrap(), "custom_name");
        assert_eq!(result[0].field_type, "i32");
        assert_eq!(result[0].kinds, vec![GetterKind::ReadOnly]);
    }

    #[test]
    #[should_panic(
        expected = "Expected format: #[common_field([all|own|own_only|mut|mut_only] field_name [as getter_name]: Type)]"
    )]
    fn test_invalid_common_field_format() {
        let input: DeriveInput = parse_quote! {
            #[derive(Debug)]
            #[common_field = "field1: i32"]
            enum TestEnum {
                Variant1 { field1: i32 },
                Variant2 { field1: i32 },
            }
        };

        parse_common_fields_attributes(&input);
    }
}
