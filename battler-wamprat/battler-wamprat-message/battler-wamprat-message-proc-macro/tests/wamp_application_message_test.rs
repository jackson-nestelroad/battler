use battler_wamprat_message::{
    Dictionary,
    Integer,
    List,
    Value,
    WampApplicationMessage,
};
use battler_wamprat_message_proc_macro::{
    WampDictionary,
    WampList,
};

#[test]
fn serializes_arguments_and_arguments_keyword() {
    #[derive(WampList)]
    struct Args {
        a: Integer,
        b: Integer,
    }

    #[derive(WampDictionary)]
    struct Options {
        dry_run: bool,
        agent: String,
    }

    #[derive(WampApplicationMessage)]
    struct Output {
        #[arguments]
        args: Args,
        #[arguments_keyword]
        options: Options,
    }

    let output = Output {
        args: Args { a: 123, b: 456 },
        options: Options {
            dry_run: true,
            agent: "battler".to_owned(),
        },
    };

    assert_matches::assert_matches!(output.wamp_serialize_application_message(), Ok((arguments, arguments_keyword)) => {
        pretty_assertions::assert_eq!(arguments, List::from_iter([
            Value::Integer(123),
            Value::Integer(456),
        ]));
        pretty_assertions::assert_eq!(arguments_keyword, Dictionary::from_iter([
            ("dry_run".to_owned(), Value::Bool(true)),
            ("agent".to_owned(), Value::String("battler".to_owned())),
        ]));
    });
}

#[test]
fn deserializes_arguments_and_arguments_keyword() {
    #[derive(Debug, PartialEq, WampList)]
    struct Args {
        a: Integer,
        b: Integer,
    }

    #[derive(Debug, PartialEq, WampDictionary)]
    struct Options {
        dry_run: bool,
        agent: String,
    }

    #[derive(Debug, PartialEq, WampApplicationMessage)]
    struct Output {
        #[arguments]
        args: Args,
        #[arguments_keyword]
        options: Options,
    }

    assert_matches::assert_matches!(Output::wamp_deserialize_application_message(List::from_iter([
        Value::Integer(123),
        Value::Integer(456),
    ]), Dictionary::from_iter([
        ("dry_run".to_owned(), Value::Bool(true)),
        ("agent".to_owned(), Value::String("battler".to_owned())),
    ])), Ok(output) => {
        pretty_assertions::assert_eq!(output, Output {
            args: Args { a: 123, b: 456 },
            options: Options {
                dry_run: true,
                agent: "battler".to_owned(),
            }
        });
    });
}

#[test]
fn allows_missing_arguments_keywords() {
    #[derive(Debug, PartialEq, WampList)]
    struct Args {
        a: Integer,
        b: Integer,
    }

    #[derive(Debug, PartialEq, WampApplicationMessage)]
    struct Output {
        #[arguments]
        args: Args,
    }

    assert_matches::assert_matches!(Output {
        args: Args { a: 123, b: 456 },
    }.wamp_serialize_application_message(), Ok((arguments, arguments_keyword)) => {
        pretty_assertions::assert_eq!(arguments, List::from_iter([
            Value::Integer(123),
            Value::Integer(456),
        ]));
        pretty_assertions::assert_eq!(arguments_keyword, Dictionary::default());
    });

    assert_matches::assert_matches!(Output::wamp_deserialize_application_message(List::from_iter([
        Value::Integer(123),
        Value::Integer(456),
    ]), Dictionary::from_iter([
        ("dry_run".to_owned(), Value::Bool(true)),
        ("agent".to_owned(), Value::String("battler".to_owned())),
    ])), Ok(output) => {
        pretty_assertions::assert_eq!(output, Output {
            args: Args { a: 123, b: 456 },
        });
    });
}

#[test]
fn allows_missing_arguments() {
    #[derive(Debug, PartialEq, WampDictionary)]
    struct Options {
        dry_run: bool,
        agent: String,
    }

    #[derive(Debug, PartialEq, WampApplicationMessage)]
    struct Output {
        #[arguments_keyword]
        options: Options,
    }

    assert_matches::assert_matches!(Output {
        options: Options {
            dry_run: true,
            agent: "battler".to_owned(),
        },
    }.wamp_serialize_application_message(), Ok((arguments, arguments_keyword)) => {
        pretty_assertions::assert_eq!(arguments, List::default());
        pretty_assertions::assert_eq!(arguments_keyword, Dictionary::from_iter([
            ("dry_run".to_owned(), Value::Bool(true)),
            ("agent".to_owned(), Value::String("battler".to_owned())),
        ]));
    });

    assert_matches::assert_matches!(Output::wamp_deserialize_application_message(List::from_iter([
        Value::Integer(123),
        Value::Integer(456),
    ]), Dictionary::from_iter([
        ("dry_run".to_owned(), Value::Bool(true)),
        ("agent".to_owned(), Value::String("battler".to_owned())),
    ])), Ok(output) => {
        pretty_assertions::assert_eq!(output, Output {
            options: Options {
                dry_run: true,
                agent: "battler".to_owned(),
            },
        });
    });
}

#[test]
fn allows_empty_struct() {
    #[derive(Debug, PartialEq, WampApplicationMessage)]
    struct Output {}

    assert_matches::assert_matches!(Output {}.wamp_serialize_application_message(), Ok((arguments, arguments_keyword)) => {
        pretty_assertions::assert_eq!(arguments, List::default());
        pretty_assertions::assert_eq!(arguments_keyword, Dictionary::default());
    });

    assert_matches::assert_matches!(Output::wamp_deserialize_application_message(List::from_iter([
        Value::Integer(123),
        Value::Integer(456),
    ]), Dictionary::from_iter([
        ("dry_run".to_owned(), Value::Bool(true)),
        ("agent".to_owned(), Value::String("battler".to_owned())),
    ])), Ok(output) => {
        pretty_assertions::assert_eq!(output, Output {});
    });
}
