use battler_wamp::core::{
    match_style::MatchStyle,
    uri::WildcardUri,
};
use battler_wamprat_uri::WampUriMatcher;
use battler_wamprat_uri_proc_macro::WampUriMatcher as WampUriMatcherUnderTest;

#[test]
fn matches_and_generates_uri_with_no_fields() {
    #[derive(Debug, PartialEq, Eq, WampUriMatcherUnderTest)]
    #[uri("com.battler.fn")]
    struct TestUri {}

    assert_matches::assert_matches!(TestUri {}.wamp_generate_uri(), Ok(uri) => {
        assert_eq!(uri.as_ref(), "com.battler.fn");
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn"), Ok(uri) => {
        pretty_assertions::assert_eq!(uri, TestUri {});
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri(""), Err(err) => {
        assert_eq!(err.to_string(), "uri does not match the static pattern");
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battling"), Err(err) => {
        assert_eq!(err.to_string(), "uri does not match the static pattern");
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.method"), Err(err) => {
        assert_eq!(err.to_string(), "uri does not match the static pattern");
    });
}

#[test]
fn matches_and_generates_uri_with_named_fields() {
    #[derive(Debug, PartialEq, Eq, WampUriMatcherUnderTest)]
    #[uri("com.battler.fn.{a}.method.{b}")]
    struct TestUri {
        a: String,
        b: u64,
    }

    assert_matches::assert_matches!(TestUri { a: "foo".to_owned(), b: 256 }.wamp_generate_uri(), Ok(uri) => {
        assert_eq!(uri.as_ref(), "com.battler.fn.foo.method.256");
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.bar.method.123"), Ok(uri) => {
        pretty_assertions::assert_eq!(uri, TestUri {
            a: "bar".to_owned(),
            b: 123
        });
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri(""), Err(err) => {
        assert_eq!(err.to_string(), "expected com for component 0");
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battling"), Err(err) => {
        assert_eq!(err.to_string(), "expected battler for component 1");
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.method"), Err(err) => {
        assert_eq!(err.to_string(), "expected method for component 4");
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.abc.method.hello"), Err(err) => {
        assert_eq!(err.to_string(), "invalid component for b");
    });
}

#[test]
fn matches_and_generates_uri_with_unnamed_fields() {
    #[derive(Debug, PartialEq, Eq, WampUriMatcherUnderTest)]
    #[uri("com.battler.test.{0}.{1}.{2}")]
    struct TestUri(String, String, String);

    assert_matches::assert_matches!(TestUri("a".to_owned(), "b".to_owned(), "c".to_owned()).wamp_generate_uri(), Ok(uri) => {
        assert_eq!(uri.as_ref(), "com.battler.test.a.b.c");
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.test.z.y.x"), Ok(uri) => {
        pretty_assertions::assert_eq!(uri, TestUri("z".to_owned(), "y".to_owned(), "x".to_owned()));
    });
}

#[test]
fn matches_and_generates_prefix_uri() {
    #[derive(Debug, PartialEq, Eq, WampUriMatcherUnderTest)]
    #[uri("com.battler.fn.{a}.{b}.{rest}")]
    struct TestUri {
        a: String,
        b: u64,
        #[rest]
        rest: Vec<String>,
    }

    assert_matches::assert_matches!(TestUri { a: "foo".to_owned(), b: 256, rest: Vec::from_iter(["hello".to_owned(), "world".to_owned()]) }.wamp_generate_uri(), Ok(uri) => {
        assert_eq!(uri.as_ref(), "com.battler.fn.foo.256.hello.world");
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.bar.123"), Ok(uri) => {
        pretty_assertions::assert_eq!(uri, TestUri {
            a: "bar".to_owned(),
            b: 123,
            rest: Vec::new(),
        });
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.bar.123.a.b.c.d.e.f.g"), Ok(uri) => {
        pretty_assertions::assert_eq!(uri, TestUri {
            a: "bar".to_owned(),
            b: 123,
            rest: Vec::from_iter([
                "a".to_owned(),
                "b".to_owned(),
                "c".to_owned(),
                "d".to_owned(),
                "e".to_owned(),
                "f".to_owned(),
                "g".to_owned(),
            ]),
        });
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri(""), Err(err) => {
        assert_eq!(err.to_string(), "expected com for component 0");
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battling"), Err(err) => {
        assert_eq!(err.to_string(), "expected battler for component 1");
    });
}

#[test]
fn validates_repeated_fields() {
    #[derive(Debug, PartialEq, Eq, WampUriMatcherUnderTest)]
    #[uri("com.battler.test.{a}.{b}.{a}")]
    struct TestUri {
        a: u64,
        b: u64,
    }

    assert_matches::assert_matches!(TestUri { a: 12, b: 24 }.wamp_generate_uri(), Ok(uri) => {
        assert_eq!(uri.as_ref(), "com.battler.test.12.24.12");
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.test.1.2.3"), Err(err) => {
        assert_eq!(err.to_string(), "inconsistent value for a in component 5")
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.test.1.2.1"), Ok(uri) => {
        pretty_assertions::assert_eq!(uri, TestUri{ a: 1, b: 2 });
    });
}

#[test]
fn matches_and_generates_uri_with_regex_with_named_fields() {
    #[derive(Debug, PartialEq, Eq, WampUriMatcherUnderTest)]
    #[uri("com.battler.{abc}_to_{def}.{xyz}.{abc}.ending")]
    struct TestUri {
        abc: String,
        def: String,
        xyz: String,
    }

    assert_matches::assert_matches!(TestUri {
        abc: "one".to_owned(),
        def: "two".to_owned(),
        xyz: "three".to_owned(),
    }.wamp_generate_uri(), Ok(uri) => {
        assert_eq!(uri.as_ref(), "com.battler.one_to_two.three.one.ending");
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.four_to_five.six.four.ending"), Ok(uri) => {
        pretty_assertions::assert_eq!(uri,TestUri {
            abc: "four".to_owned(),
            def: "five".to_owned(),
            xyz: "six".to_owned(),
        });
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.one.two.three"), Err(err) => {
        assert_eq!(err.to_string(), "uri does not match the configured pattern");
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.one_to_two.three.two.ending"), Err(err) => {
        assert_eq!(err.to_string(), "inconsistent value for abc in component 4");
    });
}

#[test]
fn matches_and_generates_uri_with_regex_with_unnamed_fields() {
    #[derive(Debug, PartialEq, Eq, WampUriMatcherUnderTest)]
    #[uri("com.battler.fn.{0}add{1}")]
    struct TestUri(u32, u32);

    assert_matches::assert_matches!(TestUri(1, 2).wamp_generate_uri(), Ok(uri) => {
        assert_eq!(uri.as_ref(), "com.battler.fn.1add2");
    });

    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.27add383"), Ok(uri) => {
        pretty_assertions::assert_eq!(uri, TestUri(27, 383));
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.add"), Err(err) => {
        assert_eq!(err.to_string(), "uri does not match the configured pattern");
    });
    assert_matches::assert_matches!(TestUri::wamp_match_uri("com.battler.fn.12addddd"), Err(err) => {
        assert_eq!(err.to_string(), "invalid component for 1");
    });
}

#[test]
fn generates_match_style_and_uri_for_router() {
    #[derive(WampUriMatcherUnderTest)]
    #[uri("com.battler.uri")]
    struct StaticUri {}

    assert_matches::assert_matches!(StaticUri::match_style(), None);
    assert_eq!(
        StaticUri::uri_for_router(),
        WildcardUri::try_from("com.battler.uri").unwrap()
    );

    #[derive(WampUriMatcherUnderTest)]
    #[uri("com.battler.uri.{0}.{1}.{2}")]
    struct WildcardPrefixUri(u64, u64, u64);

    assert_matches::assert_matches!(WildcardPrefixUri::match_style(), Some(MatchStyle::Wildcard));
    assert_eq!(
        WildcardPrefixUri::uri_for_router(),
        WildcardUri::try_from("com.battler.uri...").unwrap()
    );

    #[derive(WampUriMatcherUnderTest)]
    #[uri("com.battler.uri.{0}.{1}.{2}.{3}")]
    struct PrefixUri(u64, u64, u64, #[rest] Vec<String>);

    assert_matches::assert_matches!(PrefixUri::match_style(), Some(MatchStyle::Prefix));
    assert_eq!(
        PrefixUri::uri_for_router(),
        WildcardUri::try_from("com.battler.uri").unwrap()
    );

    #[derive(WampUriMatcherUnderTest)]
    #[uri("com.battler.uri.{0}.fn.{1}abc{2}")]
    struct NotSimpleWildcardUri(u64, u64, u64);

    assert_matches::assert_matches!(
        NotSimpleWildcardUri::match_style(),
        Some(MatchStyle::Wildcard)
    );
    assert_eq!(
        NotSimpleWildcardUri::uri_for_router(),
        WildcardUri::try_from("com.battler.uri..fn.").unwrap()
    );
}
