use alloc::{
    format,
    vec::Vec,
};

use anyhow::Result;

use crate::{
    effect::fxlang::{
        effect::Program,
        statement_parser::StatementParser,
        tree,
    },
    error::{
        WrapResultError,
        general_error,
    },
};
/// A parsed program block, which should be executed as a unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedProgramBlock {
    Leaf(tree::Statement),
    Branch(Vec<ParsedProgramBlock>),
}

impl ParsedProgramBlock {
    /// The number of statements in the block.
    ///
    /// Note that this recursively looks into all blocks if this block is a branch, so this can
    /// potentially be expensive.
    pub fn len(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Branch(blocks) => blocks.iter().map(|block| block.len()).sum(),
        }
    }
}

/// A parsed version of [`Program`], which can be evaluated in the context of an ongoing battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedProgram {
    pub block: ParsedProgramBlock,
}

impl ParsedProgram {
    /// Parses a [`Program`] into several syntax trees, one per statement.
    ///
    /// The produced program is syntactically valid, but it may not be semantically valid. For
    /// instance, some operations may fail due to mismatched types (e.g., `'string' + 10`).
    pub fn from(program: &Program) -> Result<Self> {
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

    fn down_one_level(&mut self) -> Result<()> {
        if self.depth == Self::MAX_DEPTH {
            Err(general_error(format!(
                "exceeded maximum depth of {}",
                Self::MAX_DEPTH,
            )))
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

    fn down_one_line(&mut self) -> Result<()> {
        if self.line > Self::MAX_LENGTH {
            Err(general_error(format!(
                "program too long: exceeded maximum length of {}",
                Self::MAX_LENGTH,
            )))
        } else {
            self.line += 1;
            Ok(())
        }
    }

    pub fn parse(&mut self, program: &Program) -> Result<ParsedProgram> {
        let block = self.parse_program(program)?;
        match block {
            ParsedProgramBlock::Leaf(tree::Statement::Empty) => {
                return Err(general_error("program cannot be empty"));
            }
            _ => (),
        }
        let program = ParsedProgram { block };
        Ok(program)
    }

    fn parse_program(&mut self, program: &Program) -> Result<ParsedProgramBlock> {
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

    fn parse_line(&mut self, line: &str) -> Result<tree::Statement> {
        self.down_one_line()?;
        StatementParser::new(line)
            .parse()
            .wrap_error_with_format(format_args!("invalid statement on line {}", self.line))
    }
}

#[cfg(test)]
mod program_parser_test {
    use alloc::{
        borrow::ToOwned,
        boxed::Box,
        format,
        vec,
    };

    use battler_data::Fraction;
    use pretty_assertions::assert_eq;

    use crate::effect::fxlang::{
        ParsedProgram,
        ParsedProgramBlock,
        tree,
    };

    #[test]
    fn fails_empty_program() {
        assert_matches::assert_matches!(
            ParsedProgram::from(&serde_json::from_str(r#""""#).unwrap()),
            Err(err) => assert_eq!(format!("{err:#}"), "program cannot be empty")
        )
    }

    #[test]
    fn fails_comment_only() {
        assert_matches::assert_matches!(
            ParsedProgram::from(
                &serde_json::from_str(
                    r#"
                        " # This is a comment."
                    "#,
                )
                .unwrap(),
            ),
            Err(err) => assert_eq!(format!("{err:#}"), "program cannot be empty")
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
            )
            .unwrap(),
            ParsedProgram {
                block: ParsedProgramBlock::Leaf(tree::Statement::FunctionCall(
                    tree::FunctionCall {
                        function: tree::Identifier("function_call".to_owned()),
                        args: tree::Values(vec![]),
                    }
                ))
            }
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
            )
            .unwrap(),
            ParsedProgram {
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
            }
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
            )
            .unwrap(),
            ParsedProgram {
                block: ParsedProgramBlock::Branch(vec![
                    ParsedProgramBlock::Leaf(tree::Statement::IfStatement(tree::IfStatement(
                        tree::Expr::BinaryExpr(tree::BinaryExpr {
                            lhs: Box::new(tree::Expr::Value(tree::Value::ValueFunctionCall(
                                tree::ValueFunctionCall(tree::FunctionCall {
                                    function: tree::Identifier("rand".to_owned()),
                                    args: tree::Values(vec![
                                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                            0u64.into()
                                        )),
                                        tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(
                                            1u64.into()
                                        )),
                                    ])
                                })
                            ))),
                            rhs: vec![tree::BinaryExprRhs {
                                op: tree::Operator::Equal,
                                expr: Box::new(tree::Expr::Value(tree::Value::NumberLiteral(
                                    tree::NumberLiteral::Unsigned(0u64.into())
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
                                tree::NumberLiteral::Unsigned(20u64.into())
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
                                tree::NumberLiteral::Unsigned(40u64.into())
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
            }
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
                                            "return 2 + 2"
                                        ]
                                    ]
                                ]
                            ]
                        ]
                    "#
                )
                .unwrap()
            )
            .unwrap(),
            ParsedProgram {
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
                                        tree::Expr::BinaryExpr(tree::BinaryExpr {
                                            lhs: Box::new(tree::Expr::Value(
                                                tree::Value::NumberLiteral(
                                                    tree::NumberLiteral::Unsigned(2u64.into())
                                                )
                                            )),
                                            rhs: vec![tree::BinaryExprRhs {
                                                op: tree::Operator::Add,
                                                expr: Box::new(tree::Expr::Value(
                                                    tree::Value::NumberLiteral(
                                                        tree::NumberLiteral::Unsigned(2u64.into())
                                                    )
                                                ))
                                            }]
                                        })
                                    )))
                                )])
                            ])
                        ]),
                    ])
                ])
            }
        )
    }

    #[test]
    fn fails_maximum_depth_exceeded() {
        assert_matches::assert_matches!(
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
            Err(err) => assert_eq!(format!("{err:#}"), "exceeded maximum depth of 6")
        )
    }

    #[test]
    fn reports_invalid_statement() {
        assert_matches::assert_matches!(
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
            Err(err) => assert_eq!(format!("{err:#}"), "invalid statement on line 3: unexpected token at index 3: == (expected =)")
        )
    }
}
