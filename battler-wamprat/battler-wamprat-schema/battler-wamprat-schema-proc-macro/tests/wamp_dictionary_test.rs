use battler_wamprat_schema::{
    Dictionary,
    Integer,
    List,
    Value,
    WampDeserialize,
    WampSerialize,
};
use battler_wamprat_schema_proc_macro::WampDictionary;

#[test]
fn serializes_fields() {
    #[derive(WampDictionary)]
    struct Args {
        a: Integer,
        b: String,
        c: bool,
        d: List,
        e: Dictionary,
    }

    let args = Args {
        a: 123,
        b: "foo".to_owned(),
        c: true,
        d: List::from_iter([Value::Integer(1), Value::Integer(2)]),
        e: Dictionary::from_iter([
            ("foo".to_owned(), Value::Bool(false)),
            ("bar".to_owned(), Value::Bool(true)),
        ]),
    };

    assert_matches::assert_matches!(args.wamp_serialize(), Ok(value) => {
        pretty_assertions::assert_eq!(value, Value::Dictionary(Dictionary::from_iter([
            ("a".to_owned(), Value::Integer(123)),
            ("b".to_owned(), Value::String("foo".to_owned())),
            ("c".to_owned(), Value::Bool(true)),
            ("d".to_owned(), Value::List(List::from_iter([Value::Integer(1), Value::Integer(2)]))),
            ("e".to_owned(), Value::Dictionary(Dictionary::from_iter([
                ("foo".to_owned(), Value::Bool(false)),
                ("bar".to_owned(), Value::Bool(true)),
            ]))),
        ])));
    });
}

#[test]
fn deserializes_fields() {
    #[derive(Debug, PartialEq, WampDictionary)]
    struct Args {
        a: Integer,
        b: String,
        c: bool,
        d: List,
        e: Dictionary,
    }

    assert_matches::assert_matches!(Args::wamp_deserialize(Value::Dictionary(Dictionary::from_iter([
        ("a".to_owned(), Value::Integer(123)),
        ("b".to_owned(), Value::String("foo".to_owned())),
        ("c".to_owned(), Value::Bool(true)),
        ("d".to_owned(), Value::List(List::from_iter([Value::Integer(1), Value::Integer(2)]))),
        ("e".to_owned(), Value::Dictionary(Dictionary::from_iter([
            ("foo".to_owned(), Value::Bool(false)),
            ("bar".to_owned(), Value::Bool(true)),
        ]))),
    ]))), Ok(value) => {
        pretty_assertions::assert_eq!(value, Args {
            a: 123,
            b: "foo".to_owned(),
            c: true,
            d: List::from_iter([Value::Integer(1), Value::Integer(2)]),
            e: Dictionary::from_iter([
                ("foo".to_owned(), Value::Bool(false)),
                ("bar".to_owned(), Value::Bool(true)),
            ]),
        });
    });
}

#[test]
fn allows_missing_optionals() {
    #[derive(Debug, PartialEq, WampDictionary)]
    struct Args {
        a: Integer,
        #[battler_wamprat_schema(default, skip_serializing_if = Option::is_none)]
        b: Option<Integer>,
        #[battler_wamprat_schema(default, skip_serializing_if = Option::is_none)]
        c: Option<Integer>,
    }

    assert_matches::assert_matches!(Args { a: 123, b: None, c: Some(12) }.wamp_serialize(), Ok(value) => {
        pretty_assertions::assert_eq!(value, Value::Dictionary(Dictionary::from_iter([
            ("a".to_owned(), Value::Integer(123)),
            ("c".to_owned(), Value::Integer(12)),
        ])));
    });

    assert_matches::assert_matches!(Args::wamp_deserialize(Value::Dictionary(Dictionary::from_iter([
        ("a".to_owned(), Value::Integer(123)),
        ("b".to_owned(), Value::Integer(456)),
    ]))), Ok(value) => {
        pretty_assertions::assert_eq!(value, Args {
            a: 123,
            b: Some(456),
            c: None,
        });
    });
}
