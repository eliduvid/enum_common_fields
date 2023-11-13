extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::parse::discouraged::Speculative;
use syn::parse::ParseStream;
use syn::{parse_macro_input, DeriveInput, Meta, Token};

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

/// Macro to generate getters/setters for common fields in an enum.
/// Meaning, that if every variant of your enum has some field,
/// you could access it though field() and field_mut() accessors respectively.
///
/// For example if you have common field `key` of type String you'll use the macro like this:
/// ```ignore
/// #[derive(EnumCommonFields)]
/// #[common_field(mut key: String)]
/// enum MyEnum {
///     // Some variants
/// }
/// ```
/// and will have methods `.key()` and `.key_mut()` that return `&String` and `&mut String` respectively.
/// If you don't want to create `.key_mut()` accessor, you can omit mut in the declaration like this:
/// ```ignore
/// #[derive(EnumCommonFields)]
/// #[common_field(key: String)]
/// enum MyEnum {
///     // Some variants
/// }
/// ```
#[proc_macro_derive(EnumCommonFields, attributes(common_field))]
pub fn common_fields_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let common_fields = parse_common_fields_attributes(&ast);

    if common_fields.is_empty() {
        panic!("EnumCommonFields requires at least one common_field annotation")
    }

    let enum_name = ast.ident;
    let variant_names: Vec<_> = match ast.data {
        syn::Data::Enum(e) => e.variants.into_iter().map(|v| v.ident).collect(),
        _ => panic!("EnumCommonFields can only be applied to enums"),
    };

    let mut stream = quote!();

    for CommonField {
        kinds,
        field_name,
        field_type,
        resulting_name,
    } in common_fields
    {
        if resulting_name.is_some() && kinds.len() != 1 {
            panic!("\"as getter_name\" syntax is supported only for single getter annotations (own_only, mut_only of immutable [no annotations])")
        }

        for kind in kinds {
            match kind {
                GetterKind::ReadOnly => {
                    let resulting_name =
                        resulting_name.clone().unwrap_or_else(|| field_name.clone());
                    stream.extend(quote! {
                        impl #enum_name {
                            pub fn #resulting_name(&self) -> &#field_type {
                                match self {
                                    #(Self::#variant_names(v) => &v.#field_name,)*
                                }
                            }
                        }
                    });
                }
                GetterKind::Mutable => {
                    let resulting_name = resulting_name.clone().unwrap_or_else(|| {
                        Ident::new(&format!("{field_name}_mut"), field_name.span())
                    });
                    stream.extend(quote! {
                        impl #enum_name {
                            pub fn #resulting_name(&mut self) -> &mut #field_type {
                                match self {
                                    #(Self::#variant_names(v) => &mut v.#field_name,)*
                                }
                            }
                        }
                    });
                }
                GetterKind::Owning => {
                    let resulting_name = resulting_name.clone().unwrap_or_else(|| {
                        Ident::new(&format!("into_{field_name}"), field_name.span())
                    });
                    stream.extend(quote! {
                        impl #enum_name {
                            pub fn #resulting_name(self) -> #field_type {
                                match self {
                                    #(Self::#variant_names(v) => v.#field_name,)*
                                }
                            }
                        }
                    });
                }
            }
        }
    }
    TokenStream::from(stream)
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
