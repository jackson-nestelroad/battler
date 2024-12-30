use battler_wamprat_message::{
    Dictionary,
    Integer,
    List,
    Value,
    WampDeserialize,
    WampList,
    WampSerialize,
};

#[test]
fn serializes_fields_in_order() {
    #[derive(WampList)]
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
        pretty_assertions::assert_eq!(value, Value::List(List::from_iter([
            Value::Integer(123),
            Value::String("foo".to_owned()),
            Value::Bool(true),
            Value::List(List::from_iter([Value::Integer(1), Value::Integer(2)])),
            Value::Dictionary(Dictionary::from_iter([
                ("foo".to_owned(), Value::Bool(false)),
                ("bar".to_owned(), Value::Bool(true)),
            ])),
        ])));
    });
}

#[test]
fn deserializes_fields_in_order() {
    #[derive(Debug, PartialEq, WampList)]
    struct Args {
        a: Integer,
        b: String,
        c: bool,
        d: List,
        e: Dictionary,
    }

    assert_matches::assert_matches!(Args::wamp_deserialize(Value::List(List::from_iter([
        Value::Integer(123),
        Value::String("foo".to_owned()),
        Value::Bool(true),
        Value::List(List::from_iter([Value::Integer(1), Value::Integer(2)])),
        Value::Dictionary(Dictionary::from_iter([
            ("foo".to_owned(), Value::Bool(false)),
            ("bar".to_owned(), Value::Bool(true)),
        ])),
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
    #[derive(Debug, PartialEq, WampList)]
    struct Args {
        a: Integer,
        #[battler_wamprat_message(default, skip_serializing_if = Option::is_none)]
        b: Option<Integer>,
        #[battler_wamprat_message(default, skip_serializing_if = Option::is_none)]
        c: Option<Integer>,
    }

    assert_matches::assert_matches!(Args { a: 123, b: None, c: Some(12) }.wamp_serialize(), Ok(value) => {
        pretty_assertions::assert_eq!(value, Value::List(List::from_iter([
            Value::Integer(123),
        ])));
    });

    assert_matches::assert_matches!(Args::wamp_deserialize(Value::List(List::from_iter([
        Value::Integer(123),
    ]))), Ok(value) => {
        pretty_assertions::assert_eq!(value, Args {
            a: 123,
            b: None,
            c: None,
        });
    });
}
