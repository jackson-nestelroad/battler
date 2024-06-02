use crate::{
    battler_error,
    common::{
        Error,
        Fraction,
        WrapResultError,
    },
    effect::fxlang::tree,
};

/// Tokens that are parsed from the input stream.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token {
    Identifier,
    UnquotedString,
    String,
    Integer,
    VariableStart,
    Colon,
    Comma,
    Dot,
    LeftParenthesis,
    RightParenthesis,
    LeftBracket,
    RightBracket,
    LineCommentStart,

    Assignment,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Plus,
    Minus,
    Asterisk,
    ForwardSlash,
    Percent,
    Exclamation,

    TrueKeyword,
    FalseKeyword,
    ExprKeyword,
    FuncCallKeyword,
    IfKeyword,
    ElseKeyword,
    ForEachKeyword,
    InKeyword,
    ReturnKeyword,
    OrKeyword,
    AndKeyword,
    HasKeyword,
    HasAnyKeyword,
    StrKeyword,
}

mod byte {
    pub fn valid_identifier_start(b: u8) -> bool {
        (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z')
    }

    pub fn valid_identifier(b: u8) -> bool {
        valid_identifier_start(b) || is_digit(b) || b == b'-' || b == b'_'
    }

    pub fn valid_unquoted_string(b: u8) -> bool {
        valid_identifier(b) || b == b':'
    }

    pub fn is_digit(b: u8) -> bool {
        b >= b'0' && b <= b'9'
    }

    pub fn is_whitespace(b: u8) -> bool {
        b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' || b == 11 || b == 12
    }
}

/// Context for reading the next token.
pub(crate) struct NextTokenContext {
    /// Whether or not to allow an unquoted string.
    ///
    /// An unquoted string can include characters that would normally be separated out as their own
    /// tokens. The clearest example is a colon:
    ///
    /// - `log: weather:Hail`
    ///
    /// In the above string `log` is the function name, the following colon is the function call
    /// separator, and `weather:Hail` is an unquoted string (purely for convenience).
    pub disallow_unquoted_string: bool,
}

impl NextTokenContext {
    pub fn new() -> Self {
        Self {
            disallow_unquoted_string: false,
        }
    }

    /// See [`disallow_unquoted_string`].
    pub fn with_disallow_unquoted_string(mut self, val: bool) -> Self {
        self.disallow_unquoted_string = val;
        self
    }
}

/// The result of parsing an identifier token.
///
/// If allowed, the token parser will parse unquoted strings from identifiers with some illegal
/// characters.
#[derive(PartialEq, Eq)]
enum IdentifierParseResult {
    Identifier,
    UnquotedString,
}

mod token {
    use super::{
        byte,
        IdentifierParseResult,
        NextTokenContext,
        Token,
    };
    use crate::{
        battler_error,
        common::{
            Error,
            WrapResultError,
        },
    };

    /// Parser for fxlang tokens, which are used to make up a statement.
    ///
    /// A token is parsed by the [`next_token`] method. A new token will only be parsed if the
    /// current lexeme (parsed instance of the token type) has been consumed by the caller. This
    /// allows the language parser to assume that [`next_token`] will *always* return the next
    /// unused token from the input.
    ///
    /// When a token is parsed, three things are stored in memory:
    ///
    /// 1. The [`token`] type, which is the category of token parsed. This value is used for parsing
    ///    decisions at higher levels.
    /// 1. The [`lexeme`]`, which is the parsed instance of the current token. For instance, a
    ///    [`Token::Identifier`] can have a lexeme of "foo" or "bar".
    /// 1. The [`string`], which is exclusive to the [`Token::String`] type and contains the parsed
    ///    version of a string literal. For example, the lexeme `'It\'s a great\\nparser!'` will be
    ///    parsed as `It's a great\nparser!`.
    ///
    /// The general flow of using the token parser;
    ///
    /// 1. Call [`next_token`] to get the next unused token.
    /// 1. If the token can be used, use [`consume_lexeme`] or [`consume_string`] to move the lexeme
    ///    or string out of the parser, resetting its state.
    /// 1. Move to the next state and repeat the process.
    ///
    /// Some other notes:
    ///
    /// 1. Lexemes are simply string references to a slice of the input string, so their reference
    ///    is guaranteed to be valid for the lifetime of the parser.
    /// 1. String parsing makes a separate string buffer for holding the parsed string, since it is
    ///    different from the lexeme itself. This must happen at this level because the
    ///    end-of-string delimiter can be escaped with a backslash, so tokenization depends on
    ///    string parsing.
    /// 1. Whitespace is completely ignored, except in the context of string parsing.
    /// 1. Tokens are evaluated on a byte-by-byte level. Unicode characters outside of strings will
    ///    likely not work as intended and cannot be tokenized. This is no problem because non-ASCII
    ///    characters are never used for tokens or identifiers, so they are only valid inside of
    ///    strings.
    pub(crate) struct TokenParser<'s> {
        input: &'s [u8],
        input_index: usize,
        buffer_index: usize,
        next_token: Option<Token>,
        string: Option<String>,
    }

    impl<'s> TokenParser<'s> {
        /// Creates a new token parser over the input string.
        pub fn new(input: &'s str) -> Self {
            Self {
                input: input.as_bytes(),
                input_index: 0,
                buffer_index: 0,
                next_token: None,
                string: None,
            }
        }

        fn peek_next_byte(&self) -> Option<u8> {
            self.input.get(self.buffer_index).cloned()
        }

        fn next_byte(&mut self) {
            self.buffer_index += 1;
        }

        fn put_back_byte(&mut self) -> bool {
            if self.buffer_index <= self.input_index {
                return false;
            }
            self.buffer_index -= 1;
            true
        }

        fn lexeme_buffer_empty(&self) -> bool {
            self.buffer_index == self.input_index
        }

        fn lexeme_buffer_range(&self) -> (usize, usize) {
            (self.input_index, self.buffer_index)
        }

        fn lexeme_buffer_str_from_range(&self, (start, end): (usize, usize)) -> Option<&str> {
            unsafe { Some(std::str::from_utf8_unchecked(self.input.get(start..end)?)) }
        }

        fn lexeme_buffer_str_from_range_or_default(&self, range: (usize, usize)) -> &str {
            self.lexeme_buffer_str_from_range(range).unwrap_or_default()
        }

        fn lexeme_buffer_str(&self) -> Option<&str> {
            self.lexeme_buffer_str_from_range(self.lexeme_buffer_range())
        }

        fn consume_buffer(&mut self) {
            self.input_index = self.buffer_index;
            self.string = None;
            self.next_token = None;
        }

        fn reset_buffer(&mut self) {
            self.buffer_index = self.input_index;
        }

        fn skip_whitespace_bytes(&mut self) {
            while let Some(byte) = self.peek_next_byte() {
                if !byte::is_whitespace(byte) {
                    break;
                }
                self.next_byte();
            }
            self.consume_buffer();
        }

        fn try_read_identifier(
            &mut self,
            context: &NextTokenContext,
        ) -> Option<IdentifierParseResult> {
            // Validate identifier start.
            match self.peek_next_byte() {
                Some(next) if byte::valid_identifier_start(next) => (),
                _ => return None,
            }

            // Read rest of the identifier.
            let mut result = IdentifierParseResult::Identifier;
            let mut lost_identifier_at = None;
            self.next_byte();
            while let Some(next) = self.peek_next_byte() {
                if result == IdentifierParseResult::Identifier && !byte::valid_identifier(next) {
                    if context.disallow_unquoted_string || !byte::valid_unquoted_string(next) {
                        break;
                    }
                    result = IdentifierParseResult::UnquotedString;
                    lost_identifier_at = Some(self.buffer_index);
                } else if result == IdentifierParseResult::UnquotedString
                    && !byte::valid_unquoted_string(next)
                {
                    break;
                }
                self.next_byte();
            }

            // If an unquoted string ends with a colon, we put it back, since it is most likely
            // undesired.
            if let Some(lexeme) = self.lexeme_buffer_str() {
                if lexeme.ends_with(':') {
                    self.put_back_byte();
                    if lost_identifier_at.is_some_and(|lost_index| lost_index == self.buffer_index)
                    {
                        result = IdentifierParseResult::Identifier;
                    }
                }
            }

            Some(result)
        }

        fn try_read_string(&mut self) -> Result<Option<String>, Error> {
            match self.peek_next_byte() {
                Some(b'\'') => (),
                _ => return Ok(None),
            }
            self.next_byte();
            let mut string = Vec::new();
            let mut escape = false;
            let mut terminated = false;
            while !terminated {
                match self.peek_next_byte() {
                    None => break,
                    Some(next) if escape => {
                        escape = false;
                        match next {
                            b'\'' | b'\\' => string.push(next),
                            b'n' => string.push(b'\n'),
                            _ => {
                                return Err(battler_error!(
                                    "invalid escape character: \\{}",
                                    next as char
                                ))
                            }
                        }
                    }
                    Some(b'\'') => {
                        terminated = true;
                    }
                    Some(b'\\') => {
                        if escape {
                            escape = false;
                            string.push(b'\\');
                        } else {
                            escape = true;
                        }
                    }
                    Some(next) => {
                        string.push(next);
                    }
                }
                self.next_byte();
            }

            if !terminated {
                Err(battler_error!("unterminated string"))
            } else {
                Ok(Some(unsafe {
                    std::str::from_utf8_unchecked(string.as_slice()).to_owned()
                }))
            }
        }

        fn try_read_integer(&mut self) -> bool {
            match self.peek_next_byte() {
                Some(next) if byte::is_digit(next) => (),
                _ => return false,
            }

            self.next_byte();
            while let Some(next) = self.peek_next_byte() {
                if !byte::is_digit(next) {
                    break;
                }
                self.next_byte();
            }
            true
        }

        fn try_read_symbol(&mut self) -> Option<Token> {
            match self.peek_next_byte() {
                Some(b'$') => {
                    self.next_byte();
                    Some(Token::VariableStart)
                }
                Some(b':') => {
                    self.next_byte();
                    Some(Token::Colon)
                }
                Some(b',') => {
                    self.next_byte();
                    Some(Token::Comma)
                }
                Some(b'.') => {
                    self.next_byte();
                    Some(Token::Dot)
                }
                Some(b'(') => {
                    self.next_byte();
                    Some(Token::LeftParenthesis)
                }
                Some(b')') => {
                    self.next_byte();
                    Some(Token::RightParenthesis)
                }
                Some(b'[') => {
                    self.next_byte();
                    Some(Token::LeftBracket)
                }
                Some(b']') => {
                    self.next_byte();
                    Some(Token::RightBracket)
                }
                Some(b'#') => {
                    self.next_byte();
                    Some(Token::LineCommentStart)
                }
                Some(b'=') => {
                    self.next_byte();
                    match self.peek_next_byte() {
                        Some(b'=') => {
                            self.next_byte();
                            Some(Token::Equal)
                        }
                        _ => Some(Token::Assignment),
                    }
                }
                Some(b'!') => {
                    self.next_byte();
                    match self.peek_next_byte() {
                        Some(b'=') => {
                            self.next_byte();
                            Some(Token::NotEqual)
                        }
                        _ => Some(Token::Exclamation),
                    }
                }
                Some(b'<') => {
                    self.next_byte();
                    match self.peek_next_byte() {
                        Some(b'=') => {
                            self.next_byte();
                            Some(Token::LessThanOrEqual)
                        }
                        _ => Some(Token::LessThan),
                    }
                }
                Some(b'>') => {
                    self.next_byte();
                    match self.peek_next_byte() {
                        Some(b'=') => {
                            self.next_byte();
                            Some(Token::GreaterThanOrEqual)
                        }
                        _ => Some(Token::GreaterThan),
                    }
                }
                Some(b'+') => {
                    self.next_byte();
                    Some(Token::Plus)
                }
                Some(b'-') => {
                    self.next_byte();
                    Some(Token::Minus)
                }
                Some(b'*') => {
                    self.next_byte();
                    Some(Token::Asterisk)
                }
                Some(b'/') => {
                    self.next_byte();
                    Some(Token::ForwardSlash)
                }
                Some(b'%') => {
                    self.next_byte();
                    Some(Token::Percent)
                }
                _ => None,
            }
        }

        fn identifier_to_token(
            &self,
            identifier: &str,
            parse_result: IdentifierParseResult,
        ) -> Token {
            match identifier {
                "true" => Token::TrueKeyword,
                "false" => Token::FalseKeyword,
                "expr" => Token::ExprKeyword,
                "func_call" => Token::FuncCallKeyword,
                "if" => Token::IfKeyword,
                "else" => Token::ElseKeyword,
                "foreach" => Token::ForEachKeyword,
                "in" => Token::InKeyword,
                "return" => Token::ReturnKeyword,
                "or" => Token::OrKeyword,
                "and" => Token::AndKeyword,
                "has" => Token::HasKeyword,
                "hasany" => Token::HasAnyKeyword,
                "str" => Token::StrKeyword,
                _ => match parse_result {
                    IdentifierParseResult::Identifier => Token::Identifier,
                    IdentifierParseResult::UnquotedString => Token::UnquotedString,
                },
            }
        }

        /// Returns the starting index of the current token.
        pub fn token_index(&self) -> usize {
            self.input_index
        }

        /// Returns the current token, which was the last one parsed by [`next_token`].
        pub fn token(&self) -> Option<Token> {
            self.next_token
        }

        /// Returns the current lexeme, which is the parsed instance of the current token.
        pub fn lexeme(&self) -> &str {
            self.lexeme_buffer_str().unwrap_or_default()
        }

        /// Consumes the current lexeme.
        ///
        /// The next call to [`next_token`] will parse the next token.
        pub fn consume_lexeme(&mut self) -> &str {
            let range = self.lexeme_buffer_range();
            self.consume_buffer();
            self.lexeme_buffer_str_from_range_or_default(range)
        }

        /// Consumes the current string, which is only available if the current token is
        /// [`Token::String`].
        ///
        /// The next call to [`next_token`] will parse the next token.
        pub fn consume_string(&mut self) -> Option<String> {
            let string = self.string.clone();
            self.consume_buffer();
            string
        }

        fn parse_next_token(&mut self, context: NextTokenContext) -> Result<Option<Token>, Error> {
            // Skip whitespace bytes, so that the next byte is important for the next token.
            self.skip_whitespace_bytes();

            // End of statement.
            if self.peek_next_byte().is_none() {
                return Ok(None);
            }

            // First, try to read an identifier.
            //
            // For our language, an identifier can also be an unquoted string, which has slightly
            // different semantics.
            if let Some(parse_result) = self.try_read_identifier(&context) {
                let identifier = self
                    .lexeme_buffer_str()
                    .wrap_error_with_message("parsed empty identifier")?;
                return Ok(Some(self.identifier_to_token(identifier, parse_result)));
            }
            self.reset_buffer();

            // Second, try to read an integer literal.
            if self.try_read_integer() {
                return Ok(Some(Token::Integer));
            }
            self.reset_buffer();

            // Third, try to read a symbol, which is mostly straightforward.
            if let Some(token) = self.try_read_symbol() {
                return Ok(Some(token));
            }
            self.reset_buffer();

            // Last, try to read a string.
            if let Some(string) = self.try_read_string()? {
                self.string = Some(string);
                return Ok(Some(Token::String));
            }
            self.reset_buffer();

            // At this point, we have a byte that cannot be put into any token, so the input is
            // invalid.
            match self.peek_next_byte() {
                None => return Err(battler_error!("unexpected end of line")),
                Some(next) => Err(battler_error!("unexpected character: {}", next as char)),
            }
        }

        /// Returns the next unused token, parsing a new token as needed.
        ///
        /// The next token will only be parsed if the lexeme buffer is empty. In other words, the
        /// previous lexeme parsed by the previous [`next_token`] call must have been consumed using
        /// [`consume_lexeme`] (or [`consume_string`] for strings). If it has not been consumed,
        /// this method merely returns the current token.
        pub fn next_token(&mut self, context: NextTokenContext) -> Result<Option<Token>, Error> {
            if !self.lexeme_buffer_empty() {
                return Ok(self.next_token);
            }

            self.next_token = self.parse_next_token(context)?;
            Ok(self.next_token)
        }
    }
}

/// Parser for exactly one fxlang statement, which can only span a single line.
///
/// This parser is implemented as a predictive recursive descent parser. It does not require
/// backtracking because each rule looks at the next token and predicts the correct rule to
/// use.
pub struct StatementParser<'s> {
    token_parser: token::TokenParser<'s>,
    depth: u8,
}

impl<'s> StatementParser<'s> {
    const MAX_DEPTH: u8 = 5;

    /// Creates a new statement parser over the input string.
    pub fn new(input: &'s str) -> Self {
        Self {
            token_parser: token::TokenParser::new(input),
            depth: 0,
        }
    }

    fn unexpected_token_error(&self) -> Error {
        match self.token_parser.token() {
            None => battler_error!("unexpected end of line"),
            _ => battler_error!(
                "unexpected token at index {}: {}",
                self.token_parser.token_index(),
                self.token_parser.lexeme(),
            ),
        }
    }

    fn unexpected_token_error_with_expected_hint(&self, expected: &str) -> Error {
        match self.token_parser.token() {
            None => battler_error!("unexpected end of line (expected {expected})"),
            _ => battler_error!(
                "unexpected token at index {}: {} (expected {expected})",
                self.token_parser.token_index(),
                self.token_parser.lexeme(),
            ),
        }
    }

    fn down_one_level(&mut self) -> Result<(), Error> {
        if self.depth == Self::MAX_DEPTH {
            Err(battler_error!(
                "stack overflow: exceeded maximum depth of {}",
                Self::MAX_DEPTH
            ))
        } else {
            self.depth += 1;
            Ok(())
        }
    }

    fn up_one_level(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    /// Parses a single [`tree::Statement`] from the input line.
    pub fn parse(&mut self) -> Result<tree::Statement, Error> {
        self.down_one_level()?;
        let statement = self.parse_statement()?;
        self.up_one_level();
        // Ensure we reached the end of the line.
        //
        // Empty statement means we assume the parser wants us to ignore everything.
        if self.token_parser.token().is_some() && statement != tree::Statement::Empty {
            return Err(self.unexpected_token_error());
        }
        Ok(statement)
    }

    fn parse_statement(&mut self) -> Result<tree::Statement, Error> {
        match self
            .token_parser
            .next_token(NextTokenContext::new().with_disallow_unquoted_string(true))?
        {
            None => Ok(tree::Statement::Empty),
            Some(Token::LineCommentStart) => Ok(tree::Statement::Empty),
            Some(Token::Identifier) => {
                Ok(tree::Statement::FunctionCall(self.parse_function_call()?))
            }
            Some(Token::VariableStart) => Ok(tree::Statement::Assignment(self.parse_assignment()?)),
            Some(Token::IfKeyword) => Ok(tree::Statement::IfStatement(self.parse_if_statement()?)),
            Some(Token::ElseKeyword) => Ok(tree::Statement::ElseIfStatement(
                self.parse_else_if_statement()?,
            )),
            Some(Token::ForEachKeyword) => Ok(tree::Statement::ForEachStatement(
                self.parse_for_each_statement()?,
            )),
            Some(Token::ReturnKeyword) => Ok(tree::Statement::ReturnStatement(
                self.parse_return_statement()?,
            )),
            _ => Err(self.unexpected_token_error()),
        }
    }

    fn parse_assignment(&mut self) -> Result<tree::Assignment, Error> {
        let lhs = self.parse_var()?;
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::Assignment) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("=")),
        };
        let rhs = self.parse_expr()?;
        Ok(tree::Assignment { lhs, rhs })
    }

    fn parse_if_statement(&mut self) -> Result<tree::IfStatement, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::IfKeyword) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("if")),
        };
        let expr = self.parse_expr()?;
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::Colon) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint(":")),
        };
        Ok(tree::IfStatement(expr))
    }

    fn parse_else_if_statement(&mut self) -> Result<tree::ElseIfStatement, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::ElseKeyword) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("else")),
        };
        let if_statement = match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::IfKeyword) => Some(self.parse_if_statement()?),
            Some(Token::Colon) => {
                self.token_parser.consume_lexeme();
                None
            }
            _ => return Err(self.unexpected_token_error_with_expected_hint(":")),
        };
        Ok(tree::ElseIfStatement(if_statement))
    }

    fn parse_for_each_statement(&mut self) -> Result<tree::ForEachStatement, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::ForEachKeyword) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("foreach")),
        };
        let var = self.parse_var()?;
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::InKeyword) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("in")),
        };
        let range = match self.parse_value()? {
            None => return Err(self.unexpected_token_error_with_expected_hint("value")),
            Some(value) => value,
        };
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::Colon) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint(":")),
        };
        Ok(tree::ForEachStatement { var, range })
    }

    fn parse_return_statement(&mut self) -> Result<tree::ReturnStatement, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::ReturnKeyword) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("return")),
        };
        let value = match self.parse_value()? {
            None => None,
            Some(value) => Some(value),
        };
        Ok(tree::ReturnStatement(value))
    }

    fn parse_function_call(&mut self) -> Result<tree::FunctionCall, Error> {
        let identifier = self.parse_identifier()?;
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::Colon) => self.token_parser.consume_lexeme(),
            _ => {
                return Ok(tree::FunctionCall {
                    function: identifier,
                    args: tree::Values(vec![]),
                })
            }
        };
        let values = self.parse_values(false)?;
        Ok(tree::FunctionCall {
            function: identifier,
            args: values,
        })
    }

    fn parse_identifier(&mut self) -> Result<tree::Identifier, Error> {
        match self
            .token_parser
            .next_token(NextTokenContext::new().with_disallow_unquoted_string(true))?
        {
            Some(Token::Identifier) => Ok(tree::Identifier(
                self.token_parser.consume_lexeme().to_owned(),
            )),
            _ => Err(self.unexpected_token_error_with_expected_hint("identifier")),
        }
    }

    fn parse_values(&mut self, none_allowed: bool) -> Result<tree::Values, Error> {
        let mut values = Vec::new();
        loop {
            match self.token_parser.next_token(NextTokenContext::new())? {
                None => break,
                Some(Token::Comma) => {
                    self.token_parser.consume_lexeme();
                }
                _ => match self.parse_value()? {
                    Some(value) => values.push(value),
                    None => break,
                },
            }
        }
        if values.is_empty() && !none_allowed {
            Err(battler_error!(
                "expected at least one value at index {}, found 0",
                self.token_parser.token_index()
            ))
        } else {
            Ok(tree::Values(values))
        }
    }

    fn parse_value(&mut self) -> Result<Option<tree::Value>, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::FalseKeyword | Token::TrueKeyword) => {
                Ok(Some(tree::Value::BoolLiteral(self.parse_bool_literal()?)))
            }
            Some(Token::Integer | Token::Plus | Token::Minus) => Ok(Some(
                tree::Value::NumberLiteral(self.parse_number_literal()?),
            )),
            Some(Token::String | Token::UnquotedString | Token::Identifier) => Ok(Some(
                tree::Value::StringLiteral(self.parse_string_literal()?),
            )),
            Some(Token::LeftBracket) => Ok(Some(tree::Value::List(self.parse_list()?))),
            Some(Token::VariableStart) => Ok(Some(tree::Value::Var(self.parse_var()?))),
            Some(Token::ExprKeyword) => Ok(Some(tree::Value::ValueExpr(self.parse_value_expr()?))),
            Some(Token::FuncCallKeyword) => Ok(Some(tree::Value::ValueFunctionCall(
                self.parse_value_func_call()?,
            ))),
            Some(Token::StrKeyword) => Ok(Some(tree::Value::FormattedString(
                self.parse_formatted_string()?,
            ))),
            _ => Ok(None),
        }
    }

    fn parse_bool_literal(&mut self) -> Result<tree::BoolLiteral, Error> {
        let result = match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::FalseKeyword) => {
                self.token_parser.consume_lexeme();
                tree::BoolLiteral(false)
            }
            Some(Token::TrueKeyword) => {
                self.token_parser.consume_lexeme();
                tree::BoolLiteral(true)
            }
            _ => return Err(self.unexpected_token_error_with_expected_hint("bool")),
        };
        Ok(result)
    }

    fn parse_number_literal(&mut self) -> Result<tree::NumberLiteral, Error> {
        let negative = match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::Plus) => {
                self.token_parser.consume_lexeme();
                false
            }
            Some(Token::Minus) => {
                self.token_parser.consume_lexeme();
                true
            }
            _ => false,
        };

        let mut numerator = match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::Integer) => self
                .token_parser
                .consume_lexeme()
                .parse::<i32>()
                .wrap_error_with_format(format_args!(
                    "number token \"{}\" could not parsed to integer",
                    self.token_parser.lexeme()
                ))?,
            _ => return Err(self.unexpected_token_error_with_expected_hint("integer")),
        };

        let denominator = match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::ForwardSlash) => {
                self.token_parser.consume_lexeme();
                match self.token_parser.next_token(NextTokenContext::new())? {
                    Some(Token::Integer) => self
                        .token_parser
                        .consume_lexeme()
                        .parse::<i32>()
                        .wrap_error_with_format(format_args!(
                            "number token \"{}\" could not parsed to integer",
                            self.token_parser.lexeme()
                        ))?,
                    _ => return Err(self.unexpected_token_error_with_expected_hint("integer")),
                }
            }
            _ => 1,
        };

        if negative {
            numerator = -numerator;
        }

        if numerator < 0 {
            Ok(tree::NumberLiteral::Signed(
                Fraction::new(numerator, denominator).simplify(),
            ))
        } else {
            Ok(tree::NumberLiteral::Unsigned(
                Fraction::new(numerator as u32, denominator as u32).simplify(),
            ))
        }
    }

    fn parse_string_literal(&mut self) -> Result<tree::StringLiteral, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::UnquotedString | Token::Identifier) => Ok(tree::StringLiteral(
                self.token_parser.consume_lexeme().to_owned(),
            )),
            Some(Token::String) => Ok(tree::StringLiteral(
                self.token_parser
                    .consume_string()
                    .wrap_error_with_message("string token did not produce a string")?,
            )),
            _ => return Err(self.unexpected_token_error_with_expected_hint("string")),
        }
    }

    fn parse_list(&mut self) -> Result<tree::List, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::LeftBracket) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("[")),
        };
        let values = self.parse_values(true)?;
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::RightBracket) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("]")),
        };
        Ok(tree::List(values))
    }

    fn parse_var(&mut self) -> Result<tree::Var, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::VariableStart) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("$")),
        };
        let name = self.parse_identifier()?;
        let mut member_access = Vec::new();
        while let Some(Token::Dot) = self.token_parser.next_token(NextTokenContext::new())? {
            self.token_parser.consume_lexeme();
            member_access.push(self.parse_identifier()?);
        }
        Ok(tree::Var {
            name,
            member_access,
        })
    }

    fn parse_value_expr(&mut self) -> Result<tree::ValueExpr, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::ExprKeyword) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("expr")),
        };
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::LeftParenthesis) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("(")),
        };
        self.down_one_level()?;
        let expr = self.parse_expr()?;
        self.up_one_level();
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::RightParenthesis) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint(")")),
        };
        Ok(tree::ValueExpr(Box::new(expr)))
    }

    fn parse_value_func_call(&mut self) -> Result<tree::ValueFunctionCall, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::FuncCallKeyword) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("func_call")),
        };
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::LeftParenthesis) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("(")),
        };
        self.down_one_level()?;
        let function_call = self.parse_function_call()?;
        self.up_one_level();
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::RightParenthesis) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint(")")),
        };
        Ok(tree::ValueFunctionCall(function_call))
    }

    fn parse_formatted_string(&mut self) -> Result<tree::FormattedString, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::StrKeyword) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("str")),
        };
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::LeftParenthesis) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("(")),
        };
        let template = self.parse_string_literal()?;
        let args = match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::Comma) => {
                self.token_parser.consume_lexeme();
                self.parse_values(false)?
            }
            _ => tree::Values(vec![]),
        };
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::RightParenthesis) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint(")")),
        };
        Ok(tree::FormattedString { template, args })
    }

    fn parse_expr(&mut self) -> Result<tree::Expr, Error> {
        self.parse_expr_prec_8()
    }

    fn parse_expr_prec_8(&mut self) -> Result<tree::Expr, Error> {
        let lhs = self.parse_expr_prec_7()?;
        let mut rhs = Vec::new();
        loop {
            let op = match self.token_parser.next_token(NextTokenContext::new())? {
                Some(Token::OrKeyword) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::Or
                }
                _ => break,
            };
            let expr = self.parse_expr_prec_7()?;
            rhs.push(tree::BinaryExprRhs {
                op,
                expr: Box::new(expr),
            });
        }
        if rhs.is_empty() {
            Ok(lhs)
        } else {
            Ok(tree::Expr::BinaryExpr(tree::BinaryExpr {
                lhs: Box::new(lhs),
                rhs,
            }))
        }
    }

    fn parse_expr_prec_7(&mut self) -> Result<tree::Expr, Error> {
        let lhs = self.parse_expr_prec_6()?;
        let mut rhs = Vec::new();
        loop {
            let op = match self.token_parser.next_token(NextTokenContext::new())? {
                Some(Token::AndKeyword) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::And
                }
                _ => break,
            };
            let expr = self.parse_expr_prec_6()?;
            rhs.push(tree::BinaryExprRhs {
                op,
                expr: Box::new(expr),
            });
        }
        if rhs.is_empty() {
            Ok(lhs)
        } else {
            Ok(tree::Expr::BinaryExpr(tree::BinaryExpr {
                lhs: Box::new(lhs),
                rhs,
            }))
        }
    }

    fn parse_expr_prec_6(&mut self) -> Result<tree::Expr, Error> {
        let lhs = self.parse_expr_prec_5()?;
        let mut rhs = Vec::new();
        loop {
            let op = match self.token_parser.next_token(NextTokenContext::new())? {
                Some(Token::Equal) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::Equal
                }
                Some(Token::NotEqual) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::NotEqual
                }
                _ => break,
            };
            let expr = self.parse_expr_prec_5()?;
            rhs.push(tree::BinaryExprRhs {
                op,
                expr: Box::new(expr),
            });
        }
        if rhs.is_empty() {
            Ok(lhs)
        } else {
            Ok(tree::Expr::BinaryExpr(tree::BinaryExpr {
                lhs: Box::new(lhs),
                rhs,
            }))
        }
    }

    fn parse_expr_prec_5(&mut self) -> Result<tree::Expr, Error> {
        let lhs = self.parse_expr_prec_4()?;
        let mut rhs = Vec::new();
        loop {
            let op = match self.token_parser.next_token(NextTokenContext::new())? {
                Some(Token::LessThan) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::LessThan
                }
                Some(Token::LessThanOrEqual) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::LessThanOrEqual
                }
                Some(Token::GreaterThan) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::GreaterThan
                }
                Some(Token::GreaterThanOrEqual) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::GreaterThanOrEqual
                }
                Some(Token::HasKeyword) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::Has
                }
                Some(Token::HasAnyKeyword) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::HasAny
                }
                _ => break,
            };
            let expr = self.parse_expr_prec_4()?;
            rhs.push(tree::BinaryExprRhs {
                op,
                expr: Box::new(expr),
            });
        }
        if rhs.is_empty() {
            Ok(lhs)
        } else {
            Ok(tree::Expr::BinaryExpr(tree::BinaryExpr {
                lhs: Box::new(lhs),
                rhs,
            }))
        }
    }

    fn parse_expr_prec_4(&mut self) -> Result<tree::Expr, Error> {
        let lhs = self.parse_expr_prec_3()?;
        let mut rhs = Vec::new();
        loop {
            let op = match self.token_parser.next_token(NextTokenContext::new())? {
                Some(Token::Plus) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::Add
                }
                Some(Token::Minus) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::Subtract
                }
                _ => break,
            };
            let expr = self.parse_expr_prec_3()?;
            rhs.push(tree::BinaryExprRhs {
                op,
                expr: Box::new(expr),
            });
        }
        if rhs.is_empty() {
            Ok(lhs)
        } else {
            Ok(tree::Expr::BinaryExpr(tree::BinaryExpr {
                lhs: Box::new(lhs),
                rhs,
            }))
        }
    }

    fn parse_expr_prec_3(&mut self) -> Result<tree::Expr, Error> {
        let lhs = self.parse_expr_prec_2()?;
        let mut rhs = Vec::new();
        loop {
            let op = match self.token_parser.next_token(NextTokenContext::new())? {
                Some(Token::Asterisk) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::Multiply
                }
                Some(Token::ForwardSlash) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::Divide
                }
                Some(Token::Percent) => {
                    self.token_parser.consume_lexeme();
                    tree::Operator::Modulo
                }
                _ => break,
            };
            let expr = self.parse_expr_prec_2()?;
            rhs.push(tree::BinaryExprRhs {
                op,
                expr: Box::new(expr),
            });
        }
        if rhs.is_empty() {
            Ok(lhs)
        } else {
            Ok(tree::Expr::BinaryExpr(tree::BinaryExpr {
                lhs: Box::new(lhs),
                rhs,
            }))
        }
    }

    fn parse_expr_prec_2(&mut self) -> Result<tree::Expr, Error> {
        let mut ops = Vec::new();
        loop {
            match self.token_parser.next_token(NextTokenContext::new())? {
                Some(Token::Exclamation) => {
                    self.token_parser.consume_lexeme();
                    ops.push(tree::Operator::Not);
                }
                _ => break,
            }
        }
        if ops.is_empty() {
            self.parse_expr_prec_1()
        } else {
            let expr = self.parse_expr_prec_1()?;
            Ok(tree::Expr::PrefixUnaryExpr(tree::PrefixUnaryExpr {
                ops,
                expr: Box::new(expr),
            }))
        }
    }

    fn parse_expr_prec_1(&mut self) -> Result<tree::Expr, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::LeftParenthesis) => self.parse_parenthesis_expr(),
            _ => match self.parse_value()? {
                None => Err(self.unexpected_token_error_with_expected_hint("value")),
                Some(value) => Ok(tree::Expr::Value(value)),
            },
        }
    }

    fn parse_parenthesis_expr(&mut self) -> Result<tree::Expr, Error> {
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::LeftParenthesis) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint("(")),
        };
        let expr = self.parse_expr()?;
        match self.token_parser.next_token(NextTokenContext::new())? {
            Some(Token::RightParenthesis) => self.token_parser.consume_lexeme(),
            _ => return Err(self.unexpected_token_error_with_expected_hint(")")),
        };
        Ok(expr)
    }
}

#[cfg(test)]
mod statement_parser_tests {
    use pretty_assertions::assert_eq;

    use super::StatementParser;
    use crate::{
        common::{
            assert_error_message,
            Fraction,
        },
        effect::fxlang::tree::{
            self,
            BinaryExprRhs,
        },
    };

    #[test]
    fn parses_empty_statement() {
        assert_eq!(StatementParser::new("").parse(), Ok(tree::Statement::Empty));
    }

    #[test]
    fn parses_line_comment() {
        assert_eq!(
            StatementParser::new("# This is a comment! ### 12345 'testffff").parse(),
            Ok(tree::Statement::Empty)
        );
    }

    #[test]
    fn fails_invalid_function_identifier() {
        assert_error_message(
            StatementParser::new("23456").parse(),
            "unexpected token at index 0: 23456",
        );
    }

    #[test]
    fn parses_basic_function_call() {
        assert_eq!(
            StatementParser::new("display").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("display".to_owned()),
                args: tree::Values(vec![]),
            }))
        );
    }

    #[test]
    fn parses_bool_literals() {
        assert_eq!(
            StatementParser::new("test: true false true").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("test".to_owned()),
                args: tree::Values(vec![
                    tree::Value::BoolLiteral(tree::BoolLiteral(true)),
                    tree::Value::BoolLiteral(tree::BoolLiteral(false)),
                    tree::Value::BoolLiteral(tree::BoolLiteral(true)),
                ])
            }))
        );
    }

    #[test]
    fn parses_number_literals() {
        assert_eq!(
            StatementParser::new("test: 1 -3 55/100 -1/2 +23456542 - 1   /    3").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("test".to_owned()),
                args: tree::Values(vec![
                    tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(Fraction::from(1))),
                    tree::Value::NumberLiteral(tree::NumberLiteral::Signed(Fraction::from(-3))),
                    tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(Fraction::new(
                        11, 20
                    ))),
                    tree::Value::NumberLiteral(tree::NumberLiteral::Signed(Fraction::new(-1, 2))),
                    tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(Fraction::from(
                        23456542
                    ))),
                    tree::Value::NumberLiteral(tree::NumberLiteral::Signed(Fraction::new(-1, 3))),
                ])
            }))
        );
    }

    #[test]
    fn parses_string_literals() {
        assert_eq!(
            StatementParser::new("strings: 'hello world!' 'another' 'it\\'s magnitude 7!' 'complex \\\\ backslashing \\\\\\\\ \\n :)'")
                .parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("strings".to_owned()),
                args: tree::Values(vec![
                    tree::Value::StringLiteral(tree::StringLiteral("hello world!".to_owned())),
                    tree::Value::StringLiteral(tree::StringLiteral("another".to_owned())),
                    tree::Value::StringLiteral(tree::StringLiteral("it's magnitude 7!".to_owned())),
                    tree::Value::StringLiteral(tree::StringLiteral("complex \\ backslashing \\\\ \n :)".to_owned())),
                ])
            }))
        );
    }

    #[test]
    fn parses_unquoted_string_literals() {
        assert_eq!(
            StatementParser::new("log: abcdef this-is-a-test-123 from:ability of:mon").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("log".to_owned()),
                args: tree::Values(vec![
                    tree::Value::StringLiteral(tree::StringLiteral("abcdef".to_owned())),
                    tree::Value::StringLiteral(tree::StringLiteral(
                        "this-is-a-test-123".to_owned()
                    )),
                    tree::Value::StringLiteral(tree::StringLiteral("from:ability".to_owned())),
                    tree::Value::StringLiteral(tree::StringLiteral("of:mon".to_owned())),
                ])
            }))
        );
    }

    #[test]
    fn parses_lists() {
        assert_eq!(
            StatementParser::new("lists: [1/40] [1, 2, 3] ['a' b false] []").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("lists".to_owned()),
                args: tree::Values(vec![
                    tree::Value::List(tree::List(tree::Values(vec![tree::Value::NumberLiteral(
                        tree::NumberLiteral::Unsigned(Fraction::new(1, 40))
                    )]))),
                    tree::Value::List(tree::List(tree::Values(vec![
                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(Fraction::from(
                            1
                        ))),
                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(Fraction::from(
                            2
                        ))),
                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(Fraction::from(
                            3
                        ))),
                    ]))),
                    tree::Value::List(tree::List(tree::Values(vec![
                        tree::Value::StringLiteral(tree::StringLiteral("a".to_owned())),
                        tree::Value::StringLiteral(tree::StringLiteral("b".to_owned())),
                        tree::Value::BoolLiteral(tree::BoolLiteral(false)),
                    ]))),
                    tree::Value::List(tree::List(tree::Values(vec![]))),
                ])
            }))
        );
    }

    #[test]
    fn parses_vars() {
        assert_eq!(
            StatementParser::new(
                "vars: $source $target $source.hp $a.b.c.d9.e $ident_with-000more_chars123"
            )
            .parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("vars".to_owned()),
                args: tree::Values(vec![
                    tree::Value::Var(tree::Var {
                        name: tree::Identifier("source".to_owned()),
                        member_access: vec![],
                    }),
                    tree::Value::Var(tree::Var {
                        name: tree::Identifier("target".to_owned()),
                        member_access: vec![],
                    }),
                    tree::Value::Var(tree::Var {
                        name: tree::Identifier("source".to_owned()),
                        member_access: vec![tree::Identifier("hp".to_owned())],
                    }),
                    tree::Value::Var(tree::Var {
                        name: tree::Identifier("a".to_owned()),
                        member_access: vec![
                            tree::Identifier("b".to_owned()),
                            tree::Identifier("c".to_owned()),
                            tree::Identifier("d9".to_owned()),
                            tree::Identifier("e".to_owned()),
                        ],
                    }),
                    tree::Value::Var(tree::Var {
                        name: tree::Identifier("ident_with-000more_chars123".to_owned()),
                        member_access: vec![],
                    }),
                ])
            }))
        );
    }

    #[test]
    fn parses_nested_function_calls() {
        assert_eq!(
            StatementParser::new("fn: $a func_call(rand: 1 5) $b func_call(other)").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("fn".to_owned()),
                args: tree::Values(vec![
                    tree::Value::Var(tree::Var {
                        name: tree::Identifier("a".to_owned()),
                        member_access: vec![],
                    }),
                    tree::Value::ValueFunctionCall(tree::ValueFunctionCall(tree::FunctionCall {
                        function: tree::Identifier("rand".to_owned()),
                        args: tree::Values(vec![
                            tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                Fraction::from(1)
                            )),
                            tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                Fraction::from(5)
                            )),
                        ]),
                    })),
                    tree::Value::Var(tree::Var {
                        name: tree::Identifier("b".to_owned()),
                        member_access: vec![],
                    }),
                    tree::Value::ValueFunctionCall(tree::ValueFunctionCall(tree::FunctionCall {
                        function: tree::Identifier("other".to_owned()),
                        args: tree::Values(vec![]),
                    })),
                ])
            }))
        );
    }

    #[test]
    fn fails_on_max_depth_exceeded() {
        assert_error_message(
            StatementParser::new(
                "a:func_call(b:func_call(c:func_call(d:func_call(e:func_call(f)))))",
            )
            .parse(),
            "stack overflow: exceeded maximum depth of 5",
        );
    }

    #[test]
    fn parses_simple_nested_exprs() {
        assert_eq!(
            StatementParser::new("exprs: expr(1 + 1) expr($list has ability)").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("exprs".to_owned()),
                args: tree::Values(vec![
                    tree::Value::ValueExpr(tree::ValueExpr(Box::new(tree::Expr::BinaryExpr(
                        tree::BinaryExpr {
                            lhs: Box::new(tree::Expr::Value(tree::Value::NumberLiteral(
                                tree::NumberLiteral::Unsigned(Fraction::from(1)),
                            ))),
                            rhs: vec![tree::BinaryExprRhs {
                                op: tree::Operator::Add,
                                expr: Box::new(tree::Expr::Value(tree::Value::NumberLiteral(
                                    tree::NumberLiteral::Unsigned(Fraction::from(1)),
                                ))),
                            }],
                        }
                    )))),
                    tree::Value::ValueExpr(tree::ValueExpr(Box::new(tree::Expr::BinaryExpr(
                        tree::BinaryExpr {
                            lhs: Box::new(tree::Expr::Value(tree::Value::Var(tree::Var {
                                name: tree::Identifier("list".to_owned()),
                                member_access: vec![],
                            }))),
                            rhs: vec![tree::BinaryExprRhs {
                                op: tree::Operator::Has,
                                expr: Box::new(tree::Expr::Value(tree::Value::StringLiteral(
                                    tree::StringLiteral("ability".to_owned()),
                                ))),
                            }],
                        }
                    )))),
                ])
            }))
        );
    }

    fn string_value_expr(s: &str) -> tree::Expr {
        tree::Expr::Value(tree::Value::StringLiteral(tree::StringLiteral(
            s.to_owned(),
        )))
    }

    #[test]
    #[rustfmt::skip]
    fn parses_exprs_with_operator_precedence() {
        assert_eq!(
            StatementParser::new(
                "exprs: expr(!a * b / c % d + e - f < g <= h > i >= j has k hasany l == m != n and o or p or q and r != s == t hasany u has v >= w > x <= y < z - aa + ab % ac / ad * !ae)"
            )
            .parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("exprs".to_owned()),
                args: tree::Values(vec![tree::Value::ValueExpr(tree::ValueExpr(
                    Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                        lhs: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                            lhs: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                lhs: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                    lhs: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                        lhs: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                            lhs: Box::new(tree::Expr::PrefixUnaryExpr(
                                                tree::PrefixUnaryExpr {
                                                    ops: vec![tree::Operator::Not],
                                                    expr: Box::new(string_value_expr("a")),
                                                }
                                            )),
                                            rhs: vec![
                                                tree::BinaryExprRhs {
                                                    op: tree::Operator::Multiply,
                                                    expr: Box::new(string_value_expr("b")),
                                                },
                                                tree::BinaryExprRhs {
                                                    op: tree::Operator::Divide,
                                                    expr: Box::new(string_value_expr("c")),
                                                },
                                                tree::BinaryExprRhs {
                                                    op: tree::Operator::Modulo,
                                                    expr: Box::new(string_value_expr("d")),
                                                },
                                            ],
                                        })),
                                        rhs: vec![
                                            tree::BinaryExprRhs {
                                                op: tree::Operator::Add,
                                                expr: Box::new(string_value_expr("e")),
                                            },
                                            tree::BinaryExprRhs {
                                                op: tree::Operator::Subtract,
                                                expr: Box::new(string_value_expr("f")),
                                            },
                                        ],
                                    })),
                                    rhs: vec![
                                        tree::BinaryExprRhs {
                                            op: tree::Operator::LessThan,
                                            expr: Box::new(string_value_expr("g")),
                                        },
                                        tree::BinaryExprRhs {
                                            op: tree::Operator::LessThanOrEqual,
                                            expr: Box::new(string_value_expr("h")),
                                        },
                                        tree::BinaryExprRhs {
                                            op: tree::Operator::GreaterThan,
                                            expr: Box::new(string_value_expr("i")),
                                        },
                                        tree::BinaryExprRhs {
                                            op: tree::Operator::GreaterThanOrEqual,
                                            expr: Box::new(string_value_expr("j")),
                                        },
                                        tree::BinaryExprRhs {
                                            op: tree::Operator::Has,
                                            expr: Box::new(string_value_expr("k")),
                                        },
                                        tree::BinaryExprRhs {
                                            op: tree::Operator::HasAny,
                                            expr: Box::new(string_value_expr("l")),
                                        },
                                    ],
                                })),
                                rhs: vec![
                                    tree::BinaryExprRhs {
                                        op: tree::Operator::Equal,
                                        expr: Box::new(string_value_expr("m")),
                                    },
                                    tree::BinaryExprRhs {
                                        op: tree::Operator::NotEqual,
                                        expr: Box::new(string_value_expr("n")),
                                    },
                                ],
                            })),
                            rhs: vec![
                                tree::BinaryExprRhs {
                                    op: tree::Operator::And,
                                    expr: Box::new(string_value_expr("o")),
                                },
                            ],
                        })),
                        rhs: vec![
                            tree::BinaryExprRhs {
                                op: tree::Operator::Or,
                                expr: Box::new(string_value_expr("p")),
                            },
                            tree::BinaryExprRhs {
                                op: tree::Operator::Or,
                                expr: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                    lhs: Box::new(string_value_expr("q")),
                                    rhs: vec![
                                        tree::BinaryExprRhs {
                                            op: tree::Operator::And,
                                            expr: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                                lhs: Box::new(string_value_expr("r")),
                                                rhs: vec![
                                                    tree::BinaryExprRhs {
                                                        op: tree::Operator::NotEqual,
                                                        expr: Box::new(string_value_expr("s")),
                                                    },
                                                    tree::BinaryExprRhs {
                                                        op: tree::Operator::Equal,
                                                        expr: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                                            lhs: Box::new(string_value_expr("t")),
                                                            rhs: vec![
                                                                tree::BinaryExprRhs {
                                                                    op: tree::Operator::HasAny,
                                                                    expr: Box::new(string_value_expr("u")),
                                                                },
                                                                tree::BinaryExprRhs {
                                                                    op: tree::Operator::Has,
                                                                    expr: Box::new(string_value_expr("v")),
                                                                },
                                                                tree::BinaryExprRhs {
                                                                    op: tree::Operator::GreaterThanOrEqual,
                                                                    expr: Box::new(string_value_expr("w")),
                                                                },
                                                                tree::BinaryExprRhs {
                                                                    op: tree::Operator::GreaterThan,
                                                                    expr: Box::new(string_value_expr("x")),
                                                                },
                                                                tree::BinaryExprRhs {
                                                                    op: tree::Operator::LessThanOrEqual,
                                                                    expr: Box::new(string_value_expr("y")),
                                                                },
                                                                tree::BinaryExprRhs {
                                                                    op: tree::Operator::LessThan,
                                                                    expr: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                                                        lhs: Box::new(string_value_expr("z")),
                                                                        rhs: vec![
                                                                            tree::BinaryExprRhs {
                                                                                op: tree::Operator::Subtract,
                                                                                expr: Box::new(string_value_expr("aa")),
                                                                            },
                                                                            tree::BinaryExprRhs {
                                                                                op: tree::Operator::Add,
                                                                                expr: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                                                                                    lhs: Box::new(string_value_expr("ab")),
                                                                                    rhs: vec![
                                                                                        tree::BinaryExprRhs {
                                                                                            op: tree::Operator::Modulo,
                                                                                            expr: Box::new(string_value_expr("ac")),
                                                                                        },
                                                                                        tree::BinaryExprRhs {
                                                                                            op: tree::Operator::Divide,
                                                                                            expr: Box::new(string_value_expr("ad")),
                                                                                        },
                                                                                        tree::BinaryExprRhs {
                                                                                            op: tree::Operator::Multiply,
                                                                                            expr: Box::new(tree::Expr::PrefixUnaryExpr(tree::PrefixUnaryExpr {
                                                                                                ops: vec![tree::Operator::Not],
                                                                                                expr: Box::new(string_value_expr("ae")),
                                                                                            })),
                                                                                        },
                                                                                    ],
                                                                                })),
                                                                            },
                                                                        ],
                                                                    })),
                                                                },
                                                            ],
                                                        })),
                                                    },
                                                ],
                                            })),
                                        },
                                    ],
                                })),
                            },
                        ],
                    })),
                ))])
            }))
        );
    }

    #[test]
    fn parenthesis_override_operator_precedence() {
        assert_eq!(
            StatementParser::new("exprs: expr((a + b) * c)").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("exprs".to_owned()),
                args: tree::Values(vec![tree::Value::ValueExpr(tree::ValueExpr(Box::new(
                    tree::Expr::BinaryExpr(tree::BinaryExpr {
                        lhs: Box::new(tree::Expr::BinaryExpr(tree::BinaryExpr {
                            lhs: Box::new(string_value_expr("a")),
                            rhs: vec![tree::BinaryExprRhs {
                                op: tree::Operator::Add,
                                expr: Box::new(string_value_expr("b")),
                            }],
                        })),
                        rhs: vec![BinaryExprRhs {
                            op: tree::Operator::Multiply,
                            expr: Box::new(string_value_expr("c")),
                        }],
                    })
                )))])
            }))
        );
    }

    #[test]
    fn parses_value_assignment() {
        assert_eq!(
            StatementParser::new("$var = value").parse(),
            Ok(tree::Statement::Assignment(tree::Assignment {
                lhs: tree::Var {
                    name: tree::Identifier("var".to_owned()),
                    member_access: vec![],
                },
                rhs: tree::Expr::Value(tree::Value::StringLiteral(tree::StringLiteral(
                    "value".to_owned()
                ))),
            }))
        );
    }

    #[test]
    fn parses_copy_assignment() {
        assert_eq!(
            StatementParser::new("$var = $other.prop").parse(),
            Ok(tree::Statement::Assignment(tree::Assignment {
                lhs: tree::Var {
                    name: tree::Identifier("var".to_owned()),
                    member_access: vec![],
                },
                rhs: tree::Expr::Value(tree::Value::Var(tree::Var {
                    name: tree::Identifier("other".to_owned()),
                    member_access: vec![tree::Identifier("prop".to_owned())]
                })),
            }))
        );
    }

    #[test]
    fn parses_expr_assignment() {
        assert_eq!(
            StatementParser::new("$var = 2 * func_call(rand: 1 5)").parse(),
            Ok(tree::Statement::Assignment(tree::Assignment {
                lhs: tree::Var {
                    name: tree::Identifier("var".to_owned()),
                    member_access: vec![],
                },
                rhs: tree::Expr::BinaryExpr(tree::BinaryExpr {
                    lhs: Box::new(tree::Expr::Value(tree::Value::NumberLiteral(
                        tree::NumberLiteral::Unsigned(Fraction::from(2))
                    ))),
                    rhs: vec![tree::BinaryExprRhs {
                        op: tree::Operator::Multiply,
                        expr: Box::new(tree::Expr::Value(tree::Value::ValueFunctionCall(
                            tree::ValueFunctionCall(tree::FunctionCall {
                                function: tree::Identifier("rand".to_owned()),
                                args: tree::Values(vec![
                                    tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                        Fraction::from(1)
                                    )),
                                    tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                        Fraction::from(5)
                                    )),
                                ])
                            })
                        )))
                    }]
                })
            }))
        );
    }

    #[test]
    fn parses_if_statement() {
        assert_eq!(
            StatementParser::new("if $var == 2:").parse(),
            Ok(tree::Statement::IfStatement(tree::IfStatement(
                tree::Expr::BinaryExpr(tree::BinaryExpr {
                    lhs: Box::new(tree::Expr::Value(tree::Value::Var(tree::Var {
                        name: tree::Identifier("var".to_owned()),
                        member_access: vec![],
                    }))),
                    rhs: vec![tree::BinaryExprRhs {
                        op: tree::Operator::Equal,
                        expr: Box::new(tree::Expr::Value(tree::Value::NumberLiteral(
                            tree::NumberLiteral::Unsigned(Fraction::from(2))
                        )))
                    }]
                })
            )))
        );
    }

    #[test]
    fn parses_else_statement() {
        assert_eq!(
            StatementParser::new("else:").parse(),
            Ok(tree::Statement::ElseIfStatement(tree::ElseIfStatement(
                None
            )))
        );
    }

    #[test]
    fn parses_else_if_statement() {
        assert_eq!(
            StatementParser::new("else if $val < 10:").parse(),
            Ok(tree::Statement::ElseIfStatement(tree::ElseIfStatement(
                Some(tree::IfStatement(tree::Expr::BinaryExpr(
                    tree::BinaryExpr {
                        lhs: Box::new(tree::Expr::Value(tree::Value::Var(tree::Var {
                            name: tree::Identifier("val".to_owned()),
                            member_access: vec![],
                        }))),
                        rhs: vec![tree::BinaryExprRhs {
                            op: tree::Operator::LessThan,
                            expr: Box::new(tree::Expr::Value(tree::Value::NumberLiteral(
                                tree::NumberLiteral::Unsigned(Fraction::from(10))
                            )))
                        }]
                    }
                )))
            )))
        );
    }

    #[test]
    fn parses_foreach_statement_with_var() {
        assert_eq!(
            StatementParser::new("foreach $item in $list:").parse(),
            Ok(tree::Statement::ForEachStatement(tree::ForEachStatement {
                var: tree::Var {
                    name: tree::Identifier("item".to_owned()),
                    member_access: vec![],
                },
                range: tree::Value::Var(tree::Var {
                    name: tree::Identifier("list".to_owned()),
                    member_access: vec![],
                }),
            }))
        );
    }

    #[test]
    fn parses_foreach_statement_with_list() {
        assert_eq!(
            StatementParser::new("foreach $mon in [bulbasaur, charmander, squirtle]:").parse(),
            Ok(tree::Statement::ForEachStatement(tree::ForEachStatement {
                var: tree::Var {
                    name: tree::Identifier("mon".to_owned()),
                    member_access: vec![],
                },
                range: tree::Value::List(tree::List(tree::Values(vec![
                    tree::Value::StringLiteral(tree::StringLiteral("bulbasaur".to_owned())),
                    tree::Value::StringLiteral(tree::StringLiteral("charmander".to_owned())),
                    tree::Value::StringLiteral(tree::StringLiteral("squirtle".to_owned())),
                ]))),
            }))
        );
    }

    #[test]
    fn parses_foreach_statement_with_generator() {
        assert_eq!(
            StatementParser::new("foreach $mon in func_call(range: 0 10 2):").parse(),
            Ok(tree::Statement::ForEachStatement(tree::ForEachStatement {
                var: tree::Var {
                    name: tree::Identifier("mon".to_owned()),
                    member_access: vec![],
                },
                range: tree::Value::ValueFunctionCall(tree::ValueFunctionCall(
                    tree::FunctionCall {
                        function: tree::Identifier("range".to_owned()),
                        args: tree::Values(vec![
                            tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                Fraction::from(0)
                            )),
                            tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                Fraction::from(10)
                            )),
                            tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                Fraction::from(2)
                            )),
                        ])
                    }
                )),
            }))
        );
    }

    #[test]
    fn parses_return_statement() {
        assert_eq!(
            StatementParser::new("return false").parse(),
            Ok(tree::Statement::ReturnStatement(tree::ReturnStatement(
                Some(tree::Value::BoolLiteral(tree::BoolLiteral(false)))
            )))
        );
    }

    #[test]
    fn parses_formatted_string() {
        assert_eq!(
            StatementParser::new("print: str('test = {}', 1/10, 5)").parse(),
            Ok(tree::Statement::FunctionCall(tree::FunctionCall {
                function: tree::Identifier("print".to_owned()),
                args: tree::Values(vec![tree::Value::FormattedString(tree::FormattedString {
                    template: tree::StringLiteral("test = {}".to_owned()),
                    args: tree::Values(vec![
                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(Fraction::new(
                            1, 10
                        ))),
                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(Fraction::from(
                            5
                        ))),
                    ])
                })])
            }))
        );
    }

    #[test]
    fn fails_return_missing_value() {
        assert_error_message(
            StatementParser::new("return").parse(),
            "unexpected end of line (expected value)",
        )
    }

    #[test]
    fn fails_if_statement_missing_colon() {
        assert_error_message(
            StatementParser::new("if $test == expr($other / 3)").parse(),
            "unexpected end of line (expected :)",
        )
    }

    #[test]
    fn fails_else_statement_missing_colon() {
        assert_error_message(
            StatementParser::new("else").parse(),
            "unexpected end of line (expected :)",
        )
    }

    #[test]
    fn fails_foreach_missing_in() {
        assert_error_message(
            StatementParser::new("foreach $var").parse(),
            "unexpected end of line (expected in)",
        )
    }

    #[test]
    fn fails_foreach_wrong_keyword() {
        assert_error_message(
            StatementParser::new("foreach $var of $list:").parse(),
            "unexpected token at index 13: of (expected in)",
        )
    }
}
