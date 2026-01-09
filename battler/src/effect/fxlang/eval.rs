use alloc::{
    collections::VecDeque,
    format,
    string::String,
    vec::Vec,
};

use anyhow::Result;

use crate::{
    effect::fxlang::{
        BattleEvent,
        CallbackFlag,
        DynamicEffectStateConnector,
        EvaluationContext,
        EventState,
        MaybeReferenceValue,
        MaybeReferenceValueForOperation,
        ParsedProgramBlock,
        Value,
        ValueType,
        Variable,
        VariableMut,
        VariableRegistry,
        parsed_effect::ParsedCallback,
        run_function,
        tree,
    },
    error::{
        WrapOptionError,
        WrapResultError,
        general_error,
    },
};

/// Input variables to an fxlang program.
///
/// Values are assigned to a named variable based on the [`BattleEvent`] configuration.
#[derive(Clone, Default)]
pub struct VariableInput {
    values: Vec<Value>,
}

impl VariableInput {
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.values.get_mut(index)
    }
}

impl FromIterator<Value> for VariableInput {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for VariableInput {
    type Item = Value;
    type IntoIter = <Vec<Value> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

/// Context for executing a [`ParsedProgramBlock`] over a list.
///
/// The list itself must be evaluated once at the beginning of the loop.
struct ExecuteProgramBlockOverListContext<'program> {
    item: &'program str,
    list: &'program tree::Value,
}

impl<'eval, 'program> ExecuteProgramBlockOverListContext<'program> {
    fn new(item: &'program str, list: &'program tree::Value) -> Self {
        Self { item, list }
    }
}

/// The evaluation state of a [`ParsedProgramBlock`].
struct ProgramBlockEvalState<'program> {
    skip_next_block: bool,
    last_if_statement_result: Option<bool>,
    for_each_context: Option<ExecuteProgramBlockOverListContext<'program>>,
}

impl ProgramBlockEvalState<'_> {
    fn new() -> Self {
        Self {
            skip_next_block: false,
            last_if_statement_result: None,
            for_each_context: None,
        }
    }
}

/// The result of evaluating a [`ParsedProgramBlock`].
enum ProgramStatementEvalResult<'program> {
    None,
    Skipped,
    IfStatement(bool),
    ElseIfStatement(bool),
    ForEachStatement(&'program str, &'program tree::Value),
    ReturnStatement(Option<Value>),
    ContinueStatement,
    BreakStatement,
}

/// The result of evaluating a [`ParsedProgram`].
#[derive(Default)]
pub struct ProgramEvalResult {
    pub value: Option<Value>,
}

impl ProgramEvalResult {
    pub fn new(value: Option<Value>) -> Self {
        Self { value }
    }
}

/// An fxlang evaluator.
///
/// Holds the global state of an fxlang [`ParsedProgram`] during evaluation. Individual blocks
/// ([`ParsedProgramBlock`]) are evaluated recursively and get their own local state.
pub struct Evaluator<'event_state> {
    statement: usize,
    vars: VariableRegistry,
    event: BattleEvent,
    event_state: &'event_state EventState,
}

impl<'event_state> Evaluator<'event_state> {
    /// Creates a new evaluator.
    pub fn new(event: BattleEvent, event_state: &'event_state EventState) -> Self {
        Self {
            statement: 0,
            vars: VariableRegistry::new(),
            event,
            event_state,
        }
    }

    fn initialize_vars(
        &self,
        context: &mut EvaluationContext,
        mut input: VariableInput,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<()> {
        if let Some(effect_state_connector) = effect_state_connector {
            if effect_state_connector.exists(context.battle_context_mut())? {
                self.vars
                    .set("effect_state", Value::EffectState(effect_state_connector))?;
            }
        }

        self.vars
            .set("this", Value::Effect(context.effect_handle().clone()))?;
        self.vars.set("battle", Value::Battle)?;
        self.vars.set("field", Value::Field)?;
        self.vars.set("format", Value::Format)?;

        if self.event.has_flag(CallbackFlag::TakesGeneralMon) {
            self.vars.set(
                "mon",
                Value::Mon(
                    context
                        .target_handle()
                        .wrap_expectation("context has no mon")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesTargetMon) {
            match context.target_handle() {
                Some(target_handle) => self.vars.set("target", Value::Mon(target_handle))?,
                None => (),
            }
        }
        if self.event.has_flag(CallbackFlag::TakesSourceMon) {
            match context.source_handle() {
                Some(source_handle) => self.vars.set("source", Value::Mon(source_handle))?,
                None => (),
            }
        }
        if self.event.has_flag(CallbackFlag::TakesUserMon) {
            // The user is the target of the effect.
            self.vars.set(
                "user",
                Value::Mon(
                    context
                        .target_handle()
                        .wrap_expectation("context has no user")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesSourceTargetMon) {
            // The target is the source of the effect.
            match context.source_handle() {
                Some(source_handle) => self.vars.set("target", Value::Mon(source_handle))?,
                None => (),
            }
        }
        if self
            .event
            .has_flag(CallbackFlag::TakesEffect | CallbackFlag::TakesSourceEffect)
        {
            let effect_name = if self.event.has_flag(CallbackFlag::TakesEffect) {
                "effect"
            } else if self.event.has_flag(CallbackFlag::TakesSourceEffect) {
                "source_effect"
            } else {
                unreachable!()
            };
            self.vars.set(
                effect_name,
                Value::Effect(
                    context
                        .source_effect_handle()
                        .cloned()
                        .wrap_expectation("context has no effect")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesActiveMove) {
            self.vars.set(
                "move",
                Value::ActiveMove(
                    context
                        .source_active_move_handle()
                        .wrap_expectation("context has no active move")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesOptionalEffect) {
            if let Some(source_effect_handle) = context.source_effect_handle().cloned() {
                self.vars
                    .set("effect", Value::Effect(source_effect_handle))?;
            }
        }
        if self.event.has_flag(CallbackFlag::TakesSide) {
            self.vars.set(
                "side",
                Value::Side(
                    context
                        .side_index()
                        .wrap_expectation("context has no side")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesPlayer) {
            self.vars.set(
                "player",
                Value::Player(
                    context
                        .player_index()
                        .wrap_expectation("context has no player")?,
                ),
            )?;
        }

        // Reverse the input so we can efficiently pop elements out of it.
        input.values.reverse();
        for (i, (name, value_type, required)) in self.event.input_vars().iter().enumerate() {
            match input.values.pop() {
                None | Some(Value::Undefined) => {
                    if *required {
                        return Err(general_error(format!(
                            "missing {value_type} input at position {} for variable {name}",
                            i + 1,
                        )));
                    }
                }
                Some(value) => {
                    let real_value_type = value.value_type();
                    // Undefined means we do not enforce the type of the input variable.
                    let value = if *value_type == ValueType::Undefined {
                        value
                    } else {
                        value.convert_to(*value_type).wrap_error_with_format(format_args!("input at position {} for variable {name} of type {real_value_type} cannot be converted to {value_type}", i + 1))?
                    };
                    self.vars.set(name, value)?;
                }
            }
        }

        if !input.values.is_empty() {
            return Err(general_error(format!(
                "too many input values: found {} extra values",
                input.values.len(),
            )));
        }

        Ok(())
    }

    /// Evaluates the given program.
    pub fn evaluate_program(
        &mut self,
        context: &mut EvaluationContext,
        input: VariableInput,
        callback: &ParsedCallback,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<ProgramEvalResult> {
        self.initialize_vars(context, input, effect_state_connector)?;
        let root_state = ProgramBlockEvalState::new();
        let value = match self
            .evaluate_program_block(context, &callback.program.block, &root_state)
            .wrap_error_with_format(format_args!("error on statement {}", self.statement))?
        {
            ProgramStatementEvalResult::ReturnStatement(value) => value,
            _ => None,
        };
        if !self
            .event
            .output_type_allowed(value.as_ref().map(|val| val.value_type()))
        {
            match value {
                Some(val) => {
                    return Err(general_error(format!(
                        "{:?} cannot return a {}",
                        self.event,
                        val.value_type(),
                    )));
                }
                None => {
                    return Err(general_error(format!(
                        "{:?} must return a value",
                        self.event
                    )));
                }
            }
        }
        Ok(ProgramEvalResult::new(value))
    }

    fn evaluate_program_block<'eval, 'program>(
        &'eval mut self,
        context: &mut EvaluationContext,
        block: &'program ParsedProgramBlock,
        parent_state: &'eval ProgramBlockEvalState,
    ) -> Result<ProgramStatementEvalResult<'program>>
    where
        'program: 'eval,
    {
        match block {
            ParsedProgramBlock::Leaf(statement) => {
                self.evaluate_statement(context, statement, parent_state)
            }
            ParsedProgramBlock::Branch(blocks) => {
                if parent_state.skip_next_block {
                    self.statement += block.len() as usize;
                    return Ok(ProgramStatementEvalResult::Skipped);
                }

                if let Some(for_each_context) = &parent_state.for_each_context {
                    let list = self.resolve_value(context, for_each_context.list)?;
                    if !list.supports_list_iteration() {
                        return Err(general_error(format!(
                            "cannot iterate over a {}",
                            list.value_type()
                        )));
                    }
                    let len = list
                        .len()
                        .wrap_expectation("value supports iteration but is missing a length")?;
                    // SAFETY: We only use this immutable borrow at the beginning of each loop, at
                    // the start of each execution.
                    //
                    // This list value can only potentially contain a reference to a stored
                    // variable. If so, we are also storing the object that does runtime borrow
                    // checking, so borrow errors will trigger during evaluation.
                    let list = unsafe {
                        core::mem::transmute::<MaybeReferenceValue<'_>, MaybeReferenceValue<'_>>(
                            list,
                        )
                    };
                    for i in 0..len {
                        let current_item = list.list_index(i).wrap_expectation_with_format(format_args!(
                            "list has no element at index {i}, but length at beginning of foreach loop was {len}"
                        ))?.to_owned();
                        self.vars.set(for_each_context.item, current_item)?;
                        match self.evaluate_program_blocks_once(context, blocks.as_slice())? {
                            result @ ProgramStatementEvalResult::ReturnStatement(_) => {
                                // Early return.
                                return Ok(result);
                            }
                            ProgramStatementEvalResult::ContinueStatement => {
                                continue;
                            }
                            ProgramStatementEvalResult::BreakStatement => {
                                break;
                            }
                            _ => (),
                        }
                    }

                    return Ok(ProgramStatementEvalResult::None);
                }

                self.evaluate_program_blocks_once(context, blocks.as_slice())
            }
        }
    }

    fn evaluate_program_blocks_once<'eval, 'program>(
        &'eval mut self,
        context: &mut EvaluationContext,
        blocks: &'program [ParsedProgramBlock],
    ) -> Result<ProgramStatementEvalResult<'program>>
    where
        'program: 'eval,
    {
        let mut state = ProgramBlockEvalState::new();
        for block in blocks {
            match self.evaluate_program_block(context, block, &state)? {
                result @ ProgramStatementEvalResult::ReturnStatement(_)
                | result @ ProgramStatementEvalResult::ContinueStatement
                | result @ ProgramStatementEvalResult::BreakStatement => {
                    // Early return.
                    return Ok(result);
                }
                ProgramStatementEvalResult::None => {
                    // Reset the state.
                    state.last_if_statement_result = None;
                    state.skip_next_block = false;
                    state.for_each_context = None;
                }
                ProgramStatementEvalResult::Skipped => (),
                ProgramStatementEvalResult::IfStatement(condition_met) => {
                    state.for_each_context = None;
                    // Remember this result in case we find an associated else statement.
                    state.last_if_statement_result = Some(condition_met);
                    // Skip the next block if the condition was not met.
                    state.skip_next_block = !condition_met;
                }
                ProgramStatementEvalResult::ElseIfStatement(condition_met) => {
                    state.for_each_context = None;
                    // Only remember this result if we have evaluated an if statement before.
                    //
                    // This prevents else blocks from being run on their own, without a leading if
                    // statement.
                    if state.last_if_statement_result.is_some() {
                        state.last_if_statement_result = Some(condition_met);
                    }
                    // Skip the next block if the condition was not met.
                    //
                    // This will always be false if last_if_statement_result is true.
                    state.skip_next_block = !condition_met;
                }
                ProgramStatementEvalResult::ForEachStatement(item, list) => {
                    // Reset the state.
                    state.last_if_statement_result = None;
                    state.skip_next_block = false;
                    state.for_each_context = None;
                    // Prepare the context for the next block.
                    state.for_each_context =
                        Some(ExecuteProgramBlockOverListContext::new(item, list))
                }
            }
        }
        Ok(ProgramStatementEvalResult::None)
    }

    fn evaluate_statement<'eval, 'program>(
        &'eval mut self,
        context: &'eval mut EvaluationContext,
        statement: &'program tree::Statement,
        parent_state: &'eval ProgramBlockEvalState,
    ) -> Result<ProgramStatementEvalResult<'program>>
    where
        'program: 'eval,
    {
        self.statement += 1;
        match statement {
            tree::Statement::Empty => Ok(ProgramStatementEvalResult::None),
            tree::Statement::Assignment(assignment) => {
                let value = self.evaluate_expr(context, &assignment.rhs)?;
                // SAFETY: The value produced by the expression should be some newly generated
                // value. If it is a reference to the variable that is being assigned to, the
                // program evaluation will error out because the variable registry has runtime
                // borrow checking. Thus, we allow the context to be borrowed again.
                let value = unsafe {
                    core::mem::transmute::<MaybeReferenceValue<'_>, MaybeReferenceValue<'_>>(value)
                };
                self.assign_var(context, &assignment.lhs, value)?;
                Ok(ProgramStatementEvalResult::None)
            }
            tree::Statement::FunctionCall(statement) => {
                self.evaluate_function_call(context, &statement)?;
                Ok(ProgramStatementEvalResult::None)
            }
            tree::Statement::IfStatement(statement) => Ok(ProgramStatementEvalResult::IfStatement(
                self.evaluate_if_statement(context, statement)?,
            )),
            tree::Statement::ElseIfStatement(statement) => {
                let condition_met = if let Some(false) = parent_state.last_if_statement_result {
                    // The last if statement was false, so this else block might apply.
                    if let Some(statement) = &statement.0 {
                        self.evaluate_if_statement(context, statement)?
                    } else {
                        true
                    }
                } else {
                    // The last if statement was true (or doesn't exist), so this else block does
                    // not apply and is not evaluated, even if there is a condition.
                    false
                };
                Ok(ProgramStatementEvalResult::ElseIfStatement(condition_met))
            }
            tree::Statement::ForEachStatement(statement) => {
                if !statement.var.member_access.is_empty() {
                    return Err(general_error(format!(
                        "invalid variable in foreach statement: ${}",
                        statement.var.full_name(),
                    )));
                }
                Ok(ProgramStatementEvalResult::ForEachStatement(
                    &statement.var.name.0,
                    &statement.range,
                ))
            }
            tree::Statement::ReturnStatement(statement) => {
                let value = match &statement.0 {
                    None => None,
                    Some(expr) => Some(self.evaluate_expr(context, expr)?),
                };
                Ok(ProgramStatementEvalResult::ReturnStatement(
                    value.map(|value| value.to_owned()),
                ))
            }
            tree::Statement::Continue(_) => Ok(ProgramStatementEvalResult::ContinueStatement),
            tree::Statement::Break(_) => Ok(ProgramStatementEvalResult::BreakStatement),
        }
    }

    fn evaluate_if_statement<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        statement: &'program tree::IfStatement,
    ) -> Result<bool> {
        let condition = self.evaluate_expr(context, &statement.0)?;
        let condition = match condition.boolean() {
            Some(value) => value,
            _ => {
                return Err(general_error(format!(
                    "if statement condition must return a boolean, got {}",
                    condition.value_type(),
                )));
            }
        };
        Ok(condition)
    }

    fn evaluate_function_call<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        function_call: &'program tree::FunctionCall,
    ) -> Result<Option<MaybeReferenceValue<'eval>>>
    where
        'program: 'eval,
    {
        let args = self.resolve_values(context, &function_call.args)?;
        // Functions call code outside of the evaluator, so there can be no internal references.
        let args = args.into_iter().map(|arg| arg.to_owned()).collect();
        self.run_function(context, &function_call.function.0, args)
    }

    fn run_function<'eval, 'program>(
        &'eval self,
        context: &mut EvaluationContext,
        function_name: &'program str,
        args: VecDeque<Value>,
    ) -> Result<Option<MaybeReferenceValue<'eval>>> {
        let effect_state = self
            .vars
            .get("effect_state")?
            .map(|val| (*val).clone().effect_state().ok())
            .flatten();
        run_function(
            context,
            function_name,
            args,
            self.event,
            self.event_state,
            effect_state,
        )
        .map(|val| val.map(|val| MaybeReferenceValue::from(val)))
    }

    fn evaluate_prefix_operator<'eval>(
        op: tree::Operator,
        value: MaybeReferenceValueForOperation<'eval>,
    ) -> Result<MaybeReferenceValue<'eval>> {
        match op {
            tree::Operator::Not => value.negate(),
            tree::Operator::UnaryPlus => value.unary_plus(),
            _ => Err(general_error(format!("invalid prefix operator: {op}"))),
        }
    }

    fn evaluate_binary_operator<'eval>(
        lhs: MaybeReferenceValueForOperation<'eval>,
        op: tree::Operator,
        rhs: MaybeReferenceValueForOperation<'eval>,
    ) -> Result<MaybeReferenceValue<'eval>> {
        match op {
            tree::Operator::Exponent => lhs.pow(rhs),
            tree::Operator::Multiply => lhs.multiply(rhs),
            tree::Operator::Divide => lhs.divide(rhs),
            tree::Operator::Modulo => lhs.modulo(rhs),
            tree::Operator::Add => lhs.add(rhs),
            tree::Operator::Subtract => lhs.subtract(rhs),
            tree::Operator::LessThan => lhs.less_than(rhs),
            tree::Operator::LessThanOrEqual => lhs.less_than_or_equal(rhs),
            tree::Operator::GreaterThan => lhs.greater_than(rhs),
            tree::Operator::GreaterThanOrEqual => lhs.greater_than_or_equal(rhs),
            tree::Operator::Has => lhs.has(rhs),
            tree::Operator::HasAny => lhs.has_any(rhs),
            tree::Operator::Equal => lhs.equal(rhs),
            tree::Operator::NotEqual => lhs.not_equal(rhs),
            tree::Operator::And => lhs.and(rhs),
            tree::Operator::Or => lhs.or(rhs),
            _ => Err(general_error(format!("invalid binary operator: {op}"))),
        }
    }

    fn evaluate_expr<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        expr: &'program tree::Expr,
    ) -> Result<MaybeReferenceValue<'eval>>
    where
        'program: 'eval,
    {
        match expr {
            tree::Expr::Value(value) => self.resolve_value(context, value),
            tree::Expr::PrefixUnaryExpr(prefix_expr) => {
                let mut value = self.evaluate_expr(context, prefix_expr.expr.as_ref())?;
                for op in &prefix_expr.ops {
                    let value_for_operation = MaybeReferenceValueForOperation::from(&value);
                    let result = Self::evaluate_prefix_operator(*op, value_for_operation)?;
                    // SAFETY: `value_for_operation` was consumed by `evaluate_prefix_operator`.
                    let result = unsafe {
                        core::mem::transmute::<MaybeReferenceValue<'_>, MaybeReferenceValue<'eval>>(
                            result,
                        )
                    };
                    value = result;
                }
                Ok(value)
            }
            tree::Expr::BinaryExpr(binary_expr) => {
                let value = self.evaluate_expr(context, binary_expr.lhs.as_ref())?;
                // SAFETY: `context` is not really borrowed mutably when we hold an immutable
                // reference to some value in the battle or evaluation state.
                let mut value = unsafe {
                    core::mem::transmute::<MaybeReferenceValue<'_>, MaybeReferenceValue<'_>>(value)
                };
                for rhs_expr in &binary_expr.rhs {
                    let lhs = MaybeReferenceValueForOperation::from(&value);

                    // Short-circuiting logic.
                    //
                    // Important for cases where we might check if a variable exists before
                    // accessing it.
                    match (&lhs, rhs_expr.op) {
                        (MaybeReferenceValueForOperation::Boolean(true), tree::Operator::Or) => {
                            drop(lhs);
                            value = MaybeReferenceValue::Boolean(true);
                            continue;
                        }
                        (MaybeReferenceValueForOperation::Boolean(false), tree::Operator::And) => {
                            drop(lhs);
                            value = MaybeReferenceValue::Boolean(false);
                            continue;
                        }
                        _ => (),
                    }

                    let rhs_value = self.evaluate_expr(context, rhs_expr.expr.as_ref())?;
                    let rhs = MaybeReferenceValueForOperation::from(&rhs_value);
                    let result = Self::evaluate_binary_operator(lhs, rhs_expr.op, rhs)?;
                    // SAFETY: Both `lhs` and `rhs` were consumed by `evaluate_binary_operator`.
                    let result = unsafe {
                        core::mem::transmute::<MaybeReferenceValue<'_>, MaybeReferenceValue<'eval>>(
                            result,
                        )
                    };
                    value = result;
                }
                Ok(value)
            }
        }
    }

    fn evaluate_formatted_string<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        formatted_string: &'program tree::FormattedString,
    ) -> Result<MaybeReferenceValue<'eval>>
    where
        'program: 'eval,
    {
        let args = self.resolve_values(context, &formatted_string.args)?;
        let template = formatted_string.template.0.as_str();
        let mut string = String::new();
        string.reserve(template.len());

        let mut group = String::new();
        let mut group_start = None;
        let mut next_arg_index = 0;

        for (i, c) in template.char_indices() {
            match c {
                '{' => {
                    if i > 0 && group_start == Some(i - 1) {
                        // Two left brackets in a row result in an escape.
                        group_start = None;
                        string.push(c);
                    } else {
                        // Open a new group.
                        group_start = Some(i);
                    }
                }
                '}' if group_start.is_some() => {
                    if group.is_empty() {
                        // Use next positional argument.
                        let next_arg = args
                            .get(next_arg_index)
                            .wrap_expectation_with_format(format_args!("formatted string is missing positional argument for index {next_arg_index}"))?;
                        next_arg_index += 1;
                        group = MaybeReferenceValueForOperation::from(next_arg)
                            .for_formatted_string()?;
                    } else {
                        return Err(general_error(format!("invalid format group: {group}")));
                    }

                    // Add the replaced group to the string.
                    string.push_str(&group);

                    // Reset the state, since the group was closed.
                    group_start = None;
                    group.clear();
                }
                _ => {
                    if group_start.is_some() {
                        group.push(c);
                    } else {
                        string.push(c);
                    }
                }
            }
        }

        Ok(MaybeReferenceValue::String(string))
    }

    fn resolve_value<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        value: &'program tree::Value,
    ) -> Result<MaybeReferenceValue<'eval>>
    where
        'program: 'eval,
    {
        match value {
            tree::Value::UndefinedLiteral => Ok(MaybeReferenceValue::Undefined),
            tree::Value::BoolLiteral(bool) => Ok(MaybeReferenceValue::Boolean(bool.0)),
            tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(number)) => {
                Ok(MaybeReferenceValue::UFraction(*number))
            }
            tree::Value::NumberLiteral(tree::NumberLiteral::Signed(number)) => {
                Ok(MaybeReferenceValue::Fraction(*number))
            }
            tree::Value::StringLiteral(string) => Ok(MaybeReferenceValue::String(string.0.clone())),
            tree::Value::List(list) => Ok(MaybeReferenceValue::List(
                self.resolve_values(context, &list.0)?,
            )),
            tree::Value::Var(var) => {
                let var = self.create_var(var)?;
                Ok(MaybeReferenceValue::from(var.get(context)?))
            }
            tree::Value::ValueExpr(expr) => Ok(MaybeReferenceValue::from(
                self.evaluate_expr(context, &expr.0)?,
            )),
            tree::Value::ValueFunctionCall(function_call) => {
                match self.evaluate_function_call(context, &function_call.0)? {
                    Some(value) => Ok(MaybeReferenceValue::from(value)),
                    None => Ok(MaybeReferenceValue::Undefined),
                }
            }
            tree::Value::FormattedString(formatted_string) => {
                self.evaluate_formatted_string(context, formatted_string)
            }
        }
    }

    fn resolve_values<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        values: &'program tree::Values,
    ) -> Result<Vec<MaybeReferenceValue<'eval>>>
    where
        'program: 'eval,
    {
        let mut resolved = Vec::new();
        for value in &values.0 {
            // SAFETY: It is safe to have an immutable reference into the battle state. The
            // context is not really borrowed mutably.
            let value = self.resolve_value(context, value)?;
            let value = unsafe {
                core::mem::transmute::<MaybeReferenceValue<'_>, MaybeReferenceValue<'eval>>(value)
            };
            resolved.push(value);
        }
        Ok(resolved)
    }

    fn assign_var<'eval, 'program>(
        &'eval self,
        context: &mut EvaluationContext,
        var: &'program tree::Var,
        value: MaybeReferenceValue<'eval>,
    ) -> Result<()> {
        // Drop the reference as soon as possible, because holding it might block a mutable
        // reference to what we want to assign to.
        //
        // For instance, assigning one property of an object to another property on the same object
        // results in a borrow error without this drop.
        let owned_value = value.to_owned();
        drop(value);

        let mut runtime_var = self.create_var_mut(var)?;
        let runtime_var_ref = runtime_var.get_mut(context)?;

        runtime_var_ref
            .assign(owned_value)
            .wrap_error_with_format(format_args!("failed to assign to ${}", var.full_name()))
    }

    fn create_var<'eval, 'program>(
        &'eval self,
        var: &'program tree::Var,
    ) -> Result<Variable<'eval, 'program>>
    where
        'program: 'eval,
    {
        let value = self.vars.get(&var.name.0)?;
        let member_access = var
            .member_access
            .iter()
            .map(|ident| ident.0.as_str())
            .collect();
        Ok(Variable::new(value, member_access))
    }

    fn create_var_mut<'eval, 'program>(
        &'eval self,
        var: &'program tree::Var,
    ) -> Result<VariableMut<'eval, 'program>>
    where
        'program: 'eval,
    {
        let value = match self.vars.get_mut(&var.name.0)? {
            None => {
                self.vars.set(&var.name.0, Value::Undefined)?;
                self.vars
                    .get_mut(&var.name.0)?
                    .wrap_expectation_with_format(format_args!(
                        "variable ${} is undefined even after initialization",
                        var.name.0
                    ))?
            }
            Some(value) => value,
        };
        let member_access = var
            .member_access
            .iter()
            .map(|ident| ident.0.as_str())
            .collect();
        Ok(VariableMut::new(value, member_access))
    }
}
