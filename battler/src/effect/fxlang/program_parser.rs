use crate::{
    battler_error,
    common::{
        Error,
        WrapResultError,
    },
    effect::fxlang::{
        effect::Program,
        statement_parser::StatementParser,
        tree,
    },
};

/// A parsed program block, which should be executed as a unit.
#[derive(Debug, PartialEq, Eq)]
pub enum ParsedProgramBlock {
    Leaf(tree::Statement),
    Branch(Vec<ParsedProgramBlock>),
}

/// A parsed version of [`Program`], which can be evaluated in the context of an ongoing battle.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedProgram {
    pub block: ParsedProgramBlock,
}

impl ParsedProgram {
    /// Parses a [`Program`] into several syntax trees, one per statement.
    ///
    /// The produced program is syntactically valid, but it may not be semantically valid. For
    /// instance, some operations may fail due to mismatched types (e.g., `'string' + 10`).
    pub fn from(program: &Program) -> Result<Self, Error> {
        let mut parser = ProgramParser::new();
        parser.parse(program)
    }
}

struct ProgramParser {
    line: u16,
    depth: u8,
}

impl ProgramParser {
    const MAX_DEPTH: u8 = 6;
    const MAX_LENGTH: u16 = 999;

    fn new() -> Self {
        Self { line: 0, depth: 0 }
    }

    fn down_one_level(&mut self) -> Result<(), Error> {
        if self.depth == Self::MAX_DEPTH {
            Err(battler_error!(
                "exceeded maximum depth of {}",
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

    fn down_one_line(&mut self) -> Result<(), Error> {
        if self.line > Self::MAX_LENGTH {
            Err(battler_error!(
                "program too long: exceeded maximum length of {}",
                Self::MAX_LENGTH
            ))
        } else {
            self.line += 1;
            Ok(())
        }
    }

    pub fn parse(&mut self, program: &Program) -> Result<ParsedProgram, Error> {
        let block = self.parse_program(program)?;
        match block {
            ParsedProgramBlock::Leaf(tree::Statement::Empty) => {
                return Err(battler_error!("program cannot be empty"))
            }
            _ => (),
        }
        let program = ParsedProgram { block };
        Ok(program)
    }

    fn parse_program(&mut self, program: &Program) -> Result<ParsedProgramBlock, Error> {
        self.down_one_level()?;
        let block = match program {
            Program::Leaf(line) => {
                let statement = self.parse_line(line)?;
                ParsedProgramBlock::Leaf(statement)
            }
            Program::Branch(programs) => {
                let mut parsed = Vec::new();
                for program in programs {
                    let program = self.parse_program(program)?;
                    match program {
                        ParsedProgramBlock::Leaf(tree::Statement::Empty) => (),
                        _ => parsed.push(program),
                    }
                }
                ParsedProgramBlock::Branch(parsed)
            }
        };
        self.up_one_level();
        Ok(block)
    }

    fn parse_line(&mut self, line: &str) -> Result<tree::Statement, Error> {
        self.down_one_line()?;
        StatementParser::new(line)
            .parse()
            .wrap_error_with_format(format_args!("invalid statement on line {}", self.line))
    }
}

#[cfg(test)]
mod program_parser_tests {
    use pretty_assertions::assert_eq;

    use crate::{
        common::{
            assert_error_message,
            Fraction,
        },
        effect::fxlang::{
            tree,
            ParsedProgram,
            ParsedProgramBlock,
        },
    };

    #[test]
    fn fails_empty_program() {
        assert_error_message(
            ParsedProgram::from(&serde_json::from_str(r#""""#).unwrap()),
            "program cannot be empty",
        )
    }

    #[test]
    fn fails_comment_only() {
        assert_error_message(
            ParsedProgram::from(
                &serde_json::from_str(
                    r#"
                        " # This is a comment."
                    "#,
                )
                .unwrap(),
            ),
            "program cannot be empty",
        )
    }

    #[test]
    fn parses_one_statement() {
        assert_eq!(
            ParsedProgram::from(
                &serde_json::from_str(
                    r#"
                        "function_call"
                    "#
                )
                .unwrap()
            ),
            Ok(ParsedProgram {
                block: ParsedProgramBlock::Leaf(tree::Statement::FunctionCall(
                    tree::FunctionCall {
                        function: tree::Identifier("function_call".to_owned()),
                        args: tree::Values(vec![]),
                    }
                ))
            })
        )
    }

    #[test]
    fn parses_multiple_statements() {
        assert_eq!(
            ParsedProgram::from(
                &serde_json::from_str(
                    r#"
                        [
                            "function_1",
                            "$a = 2/5",
                            " # Comment, which should be ignored.",
                            "function_2: $a"
                        ]
                    "#
                )
                .unwrap()
            ),
            Ok(ParsedProgram {
                block: ParsedProgramBlock::Branch(vec![
                    ParsedProgramBlock::Leaf(tree::Statement::FunctionCall(tree::FunctionCall {
                        function: tree::Identifier("function_1".to_owned()),
                        args: tree::Values(vec![]),
                    })),
                    ParsedProgramBlock::Leaf(tree::Statement::Assignment(tree::Assignment {
                        lhs: tree::Var {
                            name: tree::Identifier("a".to_owned()),
                            member_access: vec![],
                        },
                        rhs: tree::Expr::Value(tree::Value::NumberLiteral(
                            tree::NumberLiteral::Unsigned(Fraction::new(2, 5))
                        ))
                    })),
                    ParsedProgramBlock::Leaf(tree::Statement::FunctionCall(tree::FunctionCall {
                        function: tree::Identifier("function_2".to_owned()),
                        args: tree::Values(vec![tree::Value::Var(tree::Var {
                            name: tree::Identifier("a".to_owned()),
                            member_access: vec![],
                        })]),
                    })),
                ])
            })
        )
    }

    #[test]
    fn parses_multiple_branches() {
        assert_eq!(
            ParsedProgram::from(
                &serde_json::from_str(
                    r#"
                        [
                            " # Example program with branches.",
                            "if func_call(rand: 0 1) == 0:",
                            [
                                "$damage = 20"
                            ],
                            "else:",
                            [
                                "$damage = 40"
                            ],
                            "damage: $target $damage"
                        ]
                    "#
                )
                .unwrap()
            ),
            Ok(ParsedProgram {
                block: ParsedProgramBlock::Branch(vec![
                    ParsedProgramBlock::Leaf(tree::Statement::IfStatement(tree::IfStatement(
                        tree::Expr::BinaryExpr(tree::BinaryExpr {
                            lhs: Box::new(tree::Expr::Value(tree::Value::ValueFunctionCall(
                                tree::ValueFunctionCall(tree::FunctionCall {
                                    function: tree::Identifier("rand".to_owned()),
                                    args: tree::Values(vec![
                                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                            Fraction::from(0)
                                        )),
                                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                            Fraction::from(1)
                                        )),
                                    ])
                                })
                            ))),
                            rhs: vec![tree::BinaryExprRhs {
                                op: tree::Operator::Equal,
                                expr: Box::new(tree::Expr::Value(tree::Value::NumberLiteral(
                                    tree::NumberLiteral::Unsigned(Fraction::from(0))
                                )))
                            }]
                        })
                    ))),
                    ParsedProgramBlock::Branch(vec![ParsedProgramBlock::Leaf(
                        tree::Statement::Assignment(tree::Assignment {
                            lhs: tree::Var {
                                name: tree::Identifier("damage".to_owned()),
                                member_access: vec![],
                            },
                            rhs: tree::Expr::Value(tree::Value::NumberLiteral(
                                tree::NumberLiteral::Unsigned(Fraction::from(20))
                            )),
                        })
                    )]),
                    ParsedProgramBlock::Leaf(tree::Statement::ElseIfStatement(
                        tree::ElseIfStatement(None)
                    )),
                    ParsedProgramBlock::Branch(vec![ParsedProgramBlock::Leaf(
                        tree::Statement::Assignment(tree::Assignment {
                            lhs: tree::Var {
                                name: tree::Identifier("damage".to_owned()),
                                member_access: vec![],
                            },
                            rhs: tree::Expr::Value(tree::Value::NumberLiteral(
                                tree::NumberLiteral::Unsigned(Fraction::from(40))
                            )),
                        })
                    )]),
                    ParsedProgramBlock::Leaf(tree::Statement::FunctionCall(tree::FunctionCall {
                        function: tree::Identifier("damage".to_owned()),
                        args: tree::Values(vec![
                            tree::Value::Var(tree::Var {
                                name: tree::Identifier("target".to_owned()),
                                member_access: vec![],
                            }),
                            tree::Value::Var(tree::Var {
                                name: tree::Identifier("damage".to_owned()),
                                member_access: vec![],
                            })
                        ])
                    }))
                ])
            })
        )
    }

    #[test]
    fn parses_nested_branches() {
        assert_eq!(
            ParsedProgram::from(
                &serde_json::from_str(
                    r#"
                        [
                            " # Example program with branches.",
                            "if true:",
                            [
                                "if true:",
                                [
                                    "foreach $mon in $team:",
                                    [
                                        "if $mon.fainted:",
                                        [
                                            "return 2"
                                        ]
                                    ]
                                ]
                            ]
                        ]
                    "#
                )
                .unwrap()
            ),
            Ok(ParsedProgram {
                block: ParsedProgramBlock::Branch(vec![
                    ParsedProgramBlock::Leaf(tree::Statement::IfStatement(tree::IfStatement(
                        tree::Expr::Value(tree::Value::BoolLiteral(tree::BoolLiteral(true)))
                    ))),
                    ParsedProgramBlock::Branch(vec![
                        ParsedProgramBlock::Leaf(tree::Statement::IfStatement(tree::IfStatement(
                            tree::Expr::Value(tree::Value::BoolLiteral(tree::BoolLiteral(true)))
                        ))),
                        ParsedProgramBlock::Branch(vec![
                            ParsedProgramBlock::Leaf(tree::Statement::ForEachStatement(
                                tree::ForEachStatement {
                                    var: tree::Var {
                                        name: tree::Identifier("mon".to_owned()),
                                        member_access: vec![],
                                    },
                                    range: tree::Value::Var(tree::Var {
                                        name: tree::Identifier("team".to_owned()),
                                        member_access: vec![],
                                    })
                                }
                            )),
                            ParsedProgramBlock::Branch(vec![
                                ParsedProgramBlock::Leaf(tree::Statement::IfStatement(
                                    tree::IfStatement(tree::Expr::Value(tree::Value::Var(
                                        tree::Var {
                                            name: tree::Identifier("mon".to_owned()),
                                            member_access: vec![tree::Identifier(
                                                "fainted".to_owned()
                                            )]
                                        }
                                    )))
                                ),),
                                ParsedProgramBlock::Branch(vec![ParsedProgramBlock::Leaf(
                                    tree::Statement::ReturnStatement(tree::ReturnStatement(Some(
                                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                            Fraction::from(2)
                                        ))
                                    )))
                                )])
                            ])
                        ]),
                    ])
                ])
            })
        )
    }

    #[test]
    fn fails_maximum_depth_exceeded() {
        assert_error_message(
            ParsedProgram::from(
                &serde_json::from_str(
                    r#"
                        [
                            "if true:",
                            [
                                "if true:",
                                [
                                    "if true:",
                                    [
                                        "if true:",
                                        [
                                            "if true:",
                                            [
                                                "if true:"
                                            ]
                                        ]
                                    ]
                                ]
                            ]
                        ]
                    "#,
                )
                .unwrap(),
            ),
            "exceeded maximum depth of 6",
        )
    }

    #[test]
    fn reports_invalid_statement() {
        assert_error_message(
            ParsedProgram::from(
                &serde_json::from_str(
                    r#"
                        [
                            " # This program doesn't compile.",
                            "if $mon.id == pikachu:",
                            [
                                "$a == test"
                            ],
                            "return true"
                        ]
                    "#,
                )
                .unwrap(),
            ),
            "invalid statement on line 3: unexpected token at index 3: == (expected =)",
        )
    }
}
