use enum_common_fields::EnumCommonFields;

#[test]
fn sanity_test() {
    #[derive(Debug, PartialEq)]
    struct Key(i32, i32);
    struct First {
        key: Key,
        ttype: String,
        _some_field: String,
    }

    struct Second {
        key: Key,
        ttype: String,
        _other_field: u64,
    }

    struct Third {
        key: Key,
        ttype: String,
    }

    #[derive(EnumCommonFields)]
    #[common_field(key: Key)]
    #[common_field(mut ttype: String)]
    enum MyEnum {
        _First(First),
        _Second(Second),
        Third(Third),
    }

    let mut e: MyEnum = MyEnum::Third(Third {
        key: Key(42, 42),
        ttype: "test".to_string(),
    });
    assert_eq!((e.key(), e.ttype()), (&Key(42, 42), &"test".to_string()));
    // *(e.key_mut()) = 67; <- we don't have .key_mut(), because we haven't put "mut" in #[common_field]
    *(e.ttype_mut()) = "new value".to_string();
    assert_eq!(
        (e.key(), e.ttype()),
        (&Key(42, 42), &"new value".to_string())
    );
}

#[test]
fn test_basic_usage_immutable_accessor() {
    struct VariantOne {
        key: String,
    }
    struct VariantTwo {
        key: String,
    }

    #[derive(EnumCommonFields)]
    #[common_field(key: String)]
    enum TestEnum {
        VariantOne(VariantOne),
        _VariantTwo(VariantTwo),
    }

    let test_enum = TestEnum::VariantOne(VariantOne {
        key: "Immutable".into(),
    });
    assert_eq!(test_enum.key(), "Immutable");
}
#[test]
fn test_basic_usage_immutable_accessor_deref_conversion() {
    struct VariantOne {
        key: String,
    }
    struct VariantTwo {
        key: String,
    }

    #[derive(EnumCommonFields)]
    #[common_field(key: str)]
    enum TestEnum {
        VariantOne(VariantOne),
        _VariantTwo(VariantTwo),
    }

    let test_enum = TestEnum::VariantOne(VariantOne {
        key: "Immutable".into(),
    });
    assert_eq!(test_enum.key(), "Immutable");
}

#[test]
fn test_mutable_accessor() {
    struct VariantOne {
        key: String,
    }

    #[derive(EnumCommonFields)]
    #[common_field(mut key: String)]
    enum TestEnum {
        VariantOne(VariantOne),
    }

    let mut test_enum = TestEnum::VariantOne(VariantOne {
        key: "Mutable".into(),
    });
    test_enum.key_mut().push_str(" Accessor");
    assert_eq!(
        match &test_enum {
            TestEnum::VariantOne(v) => &v.key,
        },
        &"Mutable Accessor".to_string()
    );
}

#[test]
fn test_owning_accessor() {
    struct VariantOne {
        key: String,
    }

    #[derive(EnumCommonFields)]
    #[common_field(own key: String)]
    enum TestEnum {
        VariantOne(VariantOne),
    }

    let test_enum = TestEnum::VariantOne(VariantOne {
        key: "Owning".into(),
    });
    let string: String = test_enum.into_key();
    assert_eq!(string, "Owning".to_string());
}

#[test]
fn test_custom_getter_names() {
    struct VariantOne {
        key: String,
    }

    #[derive(EnumCommonFields)]
    #[common_field(key as get_key: String)]
    enum TestEnum {
        VariantOne(VariantOne),
    }

    let test_enum = TestEnum::VariantOne(VariantOne {
        key: "Custom".into(),
    });
    assert_eq!(test_enum.get_key(), "Custom");
}

#[test]
fn test_multiple_common_fields() {
    struct VariantOne {
        key: String,
        value: i32,
    }

    #[derive(EnumCommonFields)]
    #[common_field(key: String)]
    #[common_field(value: i32)]
    enum TestEnum {
        VariantOne(VariantOne),
    }

    let test_enum = TestEnum::VariantOne(VariantOne {
        key: "Multiple".into(),
        value: 42,
    });
    assert_eq!(test_enum.key(), "Multiple");
    assert_eq!(test_enum.value().clone(), 42);
}

#[test]
fn test_mixed_access_modifiers() {
    struct VariantOne {
        key: String,
        value: i32,
    }

    #[derive(EnumCommonFields)]
    #[common_field(mut key: String)] // Mutable accessor
    #[common_field(own value: i32)] // Owning accessor
    enum TestEnum {
        VariantOne(VariantOne),
    }

    let mut test_enum = TestEnum::VariantOne(VariantOne {
        key: "Mixed".into(),
        value: 42,
    });
    test_enum.key_mut().push_str(" Modifiers");
    assert_eq!(test_enum.key(), "Mixed Modifiers");

    // Consuming enum instance
    assert_eq!(test_enum.into_value(), 42);
}

#[test]
fn test_multiple_aliases_for_accessor() {
    struct VariantOne {
        key: String,
    }

    #[derive(EnumCommonFields)]
    #[common_field(key: String)] // Generates accessor named key()
    #[common_field(key as k: String)] // Generates accessor named k()
    #[common_field(key as get_key: String)] // Generates accessor named get_key()
    enum TestEnum {
        VariantOne(VariantOne),
    }

    let test_enum = TestEnum::VariantOne(VariantOne {
        key: "Alias".into(),
    });

    // Testing each alias
    assert_eq!(test_enum.k(), "Alias");
    assert_eq!(test_enum.key(), "Alias");
    assert_eq!(test_enum.get_key(), "Alias");
}
