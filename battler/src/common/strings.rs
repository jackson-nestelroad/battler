use std::str::pattern::Pattern;

/// Splits the string on the first occurrence of the specified delimiter.
///
/// If found, returns the same result as [`str::split_once`]. If not found, returns the input string
/// unchanged.
pub fn split_once_optional<'a, P>(input: &'a str, delimiter: P) -> (&'a str, Option<&'a str>)
where
    P: Pattern<'a>,
{
    match input.split_once(delimiter) {
        None => (input, None),
        Some((a, b)) => (a, Some(b)),
    }
}

#[cfg(test)]
mod split_once_optional_tests {
    use crate::common::split_once_optional;

    #[test]
    fn returns_split_string() {
        assert_eq!(
            split_once_optional("abcd efg hijk", " "),
            ("abcd", Some("efg hijk"))
        );
        assert_eq!(
            split_once_optional("1,2,3,4,5", ","),
            ("1", Some("2,3,4,5"))
        );
        assert_eq!(
            split_once_optional("abcdabcdabcd", "abcd"),
            ("", Some("abcdabcd"))
        );
    }

    #[test]
    fn returns_input_string() {
        assert_eq!(
            split_once_optional("abcdefghijk", " "),
            ("abcdefghijk", None)
        );
        assert_eq!(split_once_optional("efghefgh", "abcd"), ("efghefgh", None));
    }
}
