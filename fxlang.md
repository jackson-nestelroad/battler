# fxlang

**fxlang** is the JSON-based, interpreted language used in **battler** for all battle effects.

## Background

An **effect** is anything that impacts some part of a battle, such as a move, ability, item, status, weather, field effect, and more.

An **event** is something that happens in a battle that triggers effects. Some easy examples are when a move is used, when a Mon takes damage, or when a Mon switches in.

An **event callback** is logic that runs for an individual effect for some event. One effect can have multiple event callbacks for different events.

## Motivation

Pokémon battles are complex. So many things can impact different parts of a battle: moves, abilities, items, statuses, weather, field effects, and more. Furthermore, so many different things can be impacted, from calculations (e.g., damage, type effectiveness, accuracy, etc.) to other effects (on a field, side, or individual Mon). This high complexity makes supporting 900+ moves and 180+ abilities practically impossible, especially considering how effects can conditionally impact each other.

Thus, there is a need for making battle effects easy to program for different battle events and conditions.

### Potential Solutions

We are looking for a solution that:

1. is compatible with the Rust language.
1. is easy to extend for new behavior.
1. is relatively straightforward to use, even for complex effects.

An obvious solution would be to just write different event callbacks for each effect directly in Rust. However, this solution is inflexible, not easy to extend, or straightforward to use, because new effects must be written directly in Rust and built directly into the binary. Furthermore, the battle library represents data in JSON, so effect callbacks and data would be completely separate.

Another solution is to create a large set of data fields that the battle library can understand to run the effect correctly. This solution is simple for most effects (for example, most moves deal damage). Unfortunately, it is practically impossible to generalize all 1000+ battle effects into a set of fields without many strange outliers and semantics (for example, random values cannot easily be represented in this format).

The solution we opt for is an interpreted language that can be expressed directly in JSON for different event callbacks. An interpreted language can be compatible with any programming langauge, extended for new behavior, and developed by external users with little knowledge of the battle engine itself.

## Design

All effects contain an `effects` field on their JSON data type of the type `Effects`, which describes how the effect impacts the battle.

```rs
pub struct Effects {
  callbacks: Callbacks,
  condition: Condition,
};

pub struct Callbacks {
  on_*: Program,
};

pub struct Condition {
  duration: usize,
  // Other condition-specific configuration.
  callbacks: Callbacks,
}
```

`on_*` is any event callback, where `*` is the name of the event. For example, `on_damage` runs when a Mon is damaged. `on_eat_item` runs when a Mon eats an item. `on_modify_atk` runs when a Mon's attack stat is modified. `on_duration` runs when the duration of the event must be initialized.

A `Condition` is an optional condition enabled by an effect. For example, some moves have volatile effects that result from the move itself. Fly gives invulnerability to most moves with some exceptions. Trick Room affects the entire field. Abilities can even have conditions: Slow Start halves attack and speed for the first five turns of the battle.

A `Program` is an individual event callback, represented in fxlang.

```rs
pub enum Program {
  Leaf(String),
  Branch(Vec<Program>),
}
```

A program is represented in a tree-like structure. A leaf is a single string (like a single line of code), while a branch is an array of strings (like a block of code). For example, the fxlang code for the Hail weather effect could look like:

```json
{
  "duration": 5,
  "on_duration": [
    "if $source.item == icyrock:",
    ["return 8"]
    "return 5"
  ],
  "on_field_start": [
    "log: [weather, type:Hail, from:$effect, of:$source]",
  ],
  "on_field_residual_order": 1,
  "on_field_residual": [
    "log: [weather, type:Hail, upkeep]",
    "run_event: weather"
  ],
  "on_weather": [
    "if $target.types has ice:"
    ["damage: $target 1/16"]
  ],
  "on_field_end": [
    "log: [weather, type:None]"
  ]
}
```

### Syntax

fxlang supports variables, member access, expressions, conditionals, container-range loops, and function calls.

```
Statement -> FunctionCall | Assignment | IfStatement | ElseStatement | ForEachStatement | ReturnStatement | Comment

FunctionCall -> Function (':' Values)?
Function -> Identifier
Values -> Value (' ' Value)?
Value -> Var | String | Number | Bool | List | ValueExpr
String -> '\'' (QuotedChar)* '\'' | UnquotedString
UnquotedString -> [a-zA-Z0-9_\-:]+
QuotedChar -> [^'] | '\\\''
Number -> [+-]?[0-9]+('/'[0-9]+)?
Bool -> true | false
List -> '[' ListValues ']'
ListValues -> Value (',' ListValues)?
ValueExpr -> 'expr(' Expr )'
Var -> '$' MemberAccess
MemberAccess -> Identifier ('.' MemberAccess)?
Identifier -> [a-zA-Z0-9_\-]+

Assignment -> Value '=' Expr
Expr -> ExprPrec1
ExprPrec1 -> ExprPrec2 Op1 ExprPrec2
Op1 -> 'or'
ExprPrec2 -> ExprPrec3 Op2 ExprPrec3
Op2 -> 'and'
ExprPrec3 -> ExprPrec4 Op3 ExprPrec4
Op3 -> '==' | '!='
ExprPrec4 -> ExprPrec5 Op4 ExprPrec5
Op4 -> '<' | '<=' | '>' | '>=' | 'has' | 'hasany'
ExprPrec5 -> ExprPrec6 Op5 ExprPrec6
Op5 -> '+' | '-'
ExprPrec6 -> ExprPrec7 Op6 ExprPrec7
Op6 -> '*' | '/' | '%'
ExprPrec7 -> Op7 ExprPrec8
Op7 -> '!'
ExprPrec8 -> Value | '(' Expression ')'

IfStatement -> 'if' Expression ':'
ElseStatement -> 'else:'
ForEachStatement -> 'foreach' Name 'in' Var ':'
ReturnStatement -> 'return' Value
Comment -> '#' .*
```

There are some nuances to the syntax above:

1. If, else, and foreach statements do not include their associated expression due to the limitation of the language. The parser enforces that these statements will be attached to any following branch. If a branch does not follow the statement, then the statement is ignored (it is a no-op).
1. Whitespace is ignored in all statements and expresions. String literals can be used to join multiple tokens into one argument.
1. A number is represented as an integer or a fraction.
1. For simplicity, expressions cannot be used directly as arguments to a function. The result of an expression can be assigned to a variable, which can then be passed to a function; or the expression can be evluated inline using `expr()`.

### Parsing

fxlang is parsed into a structure similar to an abstract syntax tree when loaded into the battle engine. Invalid programs receive a default program (typically a no-op, Struggle for moves).

There are several node types for an fxlang tree:

- `Operator` - Yields a value of a certain type based on some operation on two expressions.
- `Value` - Yields a value of a certain type.
- `Expression` - Yields a value of a certain type after execution of nested expressions.
- `Statement` - An executable statement that yields no value.
  - `FunctionCall` - Calls a function with a set of evaluated `Value`s.
  - `IfStatement` - Conditionally executes a branch of the program based on some expression.
  - `ElseStatement` - Executes a branch of the program if the previous `IfStatement` at the same branch did not execute.
  - `ForEachStatement` - Executes a branch of the program for each element in the given list.

Below is an example fxlang program for the move Smack Down:

```
{
    "no_copy": true,
    "on_start": [
        "$applies = false",
        "if ! $mon.grounded:",
        ["$applies = true"],
        "$fly_volatiles = [fly, bounce]",
        "if $mon.volatiles hasany $fly_volatiles:"
        [
            "$applies = true",
            "cancel_move: $mon",
            "foreach volatile in $fly_volatiles:",
            ["remove_volatile: $mon $volatile"],
            "remove_volatile: $mon twoturnmove"
        ],
        "remove_volatile_without_end: $mon magnetrise",
        "remove_volatile_without_end: $mon telekineses",
        "if ! $applies:"
        ["return false"],
        "log: [start, 'what:Smack Down']"
    ],
    "on_restart": [
        "$fly_volatiles = [fly, bounce]",
        "if $mon.volatiles hasany $fly_volatiles:"
        [
            "cancel_move: $mon",
            "foreach volatile in $fly_volatiles:",
            ["remove_volatile: $mon $volatile"],
            "remove_volatile: $mon twoturnmove",
            "log: [start, 'what:Smack Down']"
        ],
    ]
}
```

Now let's look at the fxlang tree for the `on_start` event callback:

```
- Branch:
  - Assignment:
    - Left: Var: applies
    - Right: Expr: Value: Bool: false
  - If:
    - Expr:
      - Not:
        - Expr: Var: mon.grounded
    - Branch:
      - Assignment:
        - Left: Var: applies
        - Right: Expr: Value: Bool: true
  - Assignment:
    - Left: Var: fly_volatiles
    - Right: Expr: Value: List: [fly, bounce]
  - If:
    - Expr:
      - Hasany:
        - Left: Expr: Var: mon.volatiles
        - Right: Expr: Var: fly_volatiles
    - Branch:
      - Assignment:
        - Left: Var: applies
        - Right: Expr: Value: Bool: true
      - FunctionCall:
        - Function: cancel_move
        - Arguments:
          - Value: Var: mon
      - Foreach:
        - Item: volatile
        - List: Var: fly_volatiles
        - Branch:
          - FunctionCall:
            - Function: remove_volatile
            - Arguments:
              - Value: Var: mon
              - Value: Var: volatile
      - FunctionCall:
        - Function: remove_volatile
        - Arguments:
          - Value: Var: mon
          - Value: String: twoturnmove
  - FunctionCall:
    - Function: remove_volatile_without_end
    - Arguments:
      - Value: Var: mon
      - Value: String: magnetrise
  - FunctionCall:
    - Function: remove_volatile_without_end
    - Arguments:
      - Value: Var: mon
      - Value: String: telekineses
  - If:
    - Expr:
      - Not:
        - Expr: Var: applies
    - Branch:
      - Return:
        - Value: Bool: false
  - FunctionCall:
    - Function: log
    - Arguments:
      - Value: List: [start, 'what:Smack Down']
```

The above tree should have enough context to interpret the code whenever the Smack Down move is run and starts the "smackdown" volatile effect on the target Mon.

### Interpreting

An fxlang program is interpreted in light of some `Context` object. There are several context types in the battle engine:

- `Context` - scoped to a single battle.
- `SideContext` - scoped to a single side.
- `PlayerContext` - scoped to a single player.
- `MonContext` - scoped to a single Mon.
- `ActiveMoveContext` - scoped to a single move performed by a Mon.
- `ActiveTargetContext` - scoped to a single Mon being targeted by a Mon's active move.
- `EffectContext` - scoped to a single effect.
- `ApplyingEffectContext` - scoped to an effect being applied in battle, which has a source and target.

The type of context required for a program depends on the callback type. This information is statically typed into the battle engine. In nearly all cases, the [`ApplyingEffectContext`] will be used, since it contains information about a single effect, a source Mon, and a target Mon.

Some move callbacks may take `ActiveMoveContext`, which is a specialization of `ApplyingEffectContext`.
