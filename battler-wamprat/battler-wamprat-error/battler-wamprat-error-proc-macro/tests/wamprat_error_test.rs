use std::fmt::Display;

use battler_wamp::core::error::WampError;
use battler_wamp_uri::Uri;
use battler_wamprat_error_proc_macro::WampError as WampErrorUnderTest;

#[test]
fn converts_struct_into_error() {
    #[derive(WampErrorUnderTest)]
    #[uri("com.battler.error")]
    struct Error(String);

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    assert_eq!(
        Into::<WampError>::into(Error("Hello, world!".to_owned())),
        WampError::new(Uri::try_from("com.battler.error").unwrap(), "Hello, world!")
    );
}

#[test]
fn converts_enum_into_error() {
    #[derive(WampErrorUnderTest)]
    enum Error {
        #[uri("com.battler.error.not_found")]
        NotFound,
        #[uri("com.battler.error.internal")]
        Internal,
    }

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    Self::NotFound => "not found",
                    Self::Internal => "internal",
                }
            )
        }
    }

    assert_eq!(
        Into::<WampError>::into(Error::NotFound),
        WampError::new(
            Uri::try_from("com.battler.error.not_found").unwrap(),
            "not found"
        )
    );
    assert_eq!(
        Into::<WampError>::into(Error::Internal),
        WampError::new(
            Uri::try_from("com.battler.error.internal").unwrap(),
            "internal"
        )
    );
}

#[test]
fn converts_wamp_error_into_unit_struct() {
    #[derive(Debug, PartialEq, WampErrorUnderTest)]
    #[uri("com.battler.error")]
    struct Error;

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "error")
        }
    }

    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error);
        }
    );
}

#[test]
fn converts_wamp_error_into_struct_with_no_fields_and_braces() {
    #[derive(Debug, PartialEq, WampErrorUnderTest)]
    #[uri("com.battler.error")]
    struct Error {}

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "error")
        }
    }

    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error {});
        }
    );
}

#[test]
fn converts_wamp_error_into_struct_with_no_fields_and_parenthesis() {
    #[derive(Debug, PartialEq, WampErrorUnderTest)]
    #[uri("com.battler.error")]
    struct Error();

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "error")
        }
    }

    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error());
        }
    );
}

#[test]
fn converts_wamp_error_into_struct_with_named_fields() {
    #[derive(Debug, PartialEq, WampErrorUnderTest)]
    #[uri("com.battler.error")]
    struct Error {
        msg: String,
    }

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.msg)
        }
    }

    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error {
                msg: "Hello, world!".to_owned(),
            });
        }
    );
}

#[test]
fn converts_wamp_error_into_struct_with_unnamed_fields() {
    #[derive(Debug, PartialEq, WampErrorUnderTest)]
    #[uri("com.battler.error")]
    struct Error(String);

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error("Hello, world!".to_owned()));
        }
    );
    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(
            Uri::try_from("com.battler.wrong_uri").unwrap(),
            "Hello, world!"
        )),
        Err(_)
    );
}

#[test]
fn converts_wamp_error_into_enum() {
    #[derive(Debug, PartialEq, WampErrorUnderTest)]
    enum Error {
        #[uri("com.battler.error.one")]
        One,
        #[uri("com.battler.error.two")]
        Two(),
        #[uri("com.battler.error.three")]
        Three(String),
        #[uri("com.battler.error.four")]
        Four {},
        #[uri("com.battler.error.five")]
        Five { msg: String },
    }

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "error")
        }
    }

    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error.one").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error::One);
        }
    );
    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error.two").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error::Two());
        }
    );
    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error.three").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error::Three("Hello, world!".to_owned()));
        }
    );
    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error.four").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error::Four{});
        }
    );
    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(Uri::try_from("com.battler.error.five").unwrap(), "Hello, world!")),
        Ok(err) => {
            assert_eq!(err, Error::Five{
                msg: "Hello, world!".to_owned(),
            });
        }
    );
    assert_matches::assert_matches!(
        TryInto::<Error>::try_into(WampError::new(
            Uri::try_from("com.battler.error.six").unwrap(),
            "Hello, world!"
        )),
        Err(_)
    );
}
