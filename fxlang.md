# fxlang

**fxlang** is the JSON-based interpreted language used in **battler** for all battle effects.

## Motivation

Pokémon battles are complex. Many things can impact many different parts of a battle: moves, abilities, items, statuses, volatile statuses (which can stack), weather, field effects, and more. Furthermore, many different things can be impacted by these different effects, from calculations (e.g., damage, type effectiveness, accuracy, etc.) to other effects themselves (such as those on a field, side, or individual Mon). This high complexity makes supporting 900+ moves and 180+ abilities practically impossible in the core logic of a battle engine.

Thus, there is a need for making battle effects easy to program for different battle events and conditions.

## Definitions

An **effect** is anything that impacts some part of a battle, such as a move, ability, item, status, weather, field effect, and more.

An **event** is something that happens in a battle that triggers effects to activate. Some easy examples are when a move is used, when a Mon takes damage, or when a Mon switches in.

An **event callback** is logic that runs for an individual effect on the firing of some event. One effect can have multiple event callbacks to run logic on different events.

Our goal is to allow a multitude of _effects_ to define a set of _event callbacks_ that will be triggered by battle _events_.

## Potential Solutions

We are looking for a solution that:

1. is compatible with the Rust language;
1. is easy to extend for new behavior; and
1. is relatively straightforward to use, even for complex effects.

An obvious solution would be to just write different event callbacks for each effect directly in Rust. However, this solution is inflexible and is not straightforward to use, because new effects must be written directly in Rust and built directly into the binary. Furthermore, the battle library represents data in JSON, so effect callbacks and data would be completely separate.

Another solution is to create a large set of data fields that the battle library can understand to run the effect correctly. This solution is simple for most effects (for example, most effects deal damage, and most secondary effects are simple stat changes or status effects). Unfortunately, it is practically impossible to generalize all 1000+ battle effects into a set of scalar fields without many strange outliers (for example, random values cannot easily be represented in this format). Complex moves will always require some custom programming.

The solution we opt for is an interpreted language that can be expressed direclty in JSON for different event callbacks. An interpreted language can be compatible with any programming language, extended for new behavior, and developed by external users with less knowledge of the internals of the battle engine itself (the interpreted language can hide away some complexities).

## Design

**fxlang** (short for "effects language") is a JSON-based interpreted language for writing battle effect event callbacks. When an event occurs in battle, the battle engine will gather the active effects in the battle and run any associated callbacks for the event.

### Language

Like all other data, callbacks are defined directly in JSON, allowing callbacks to be defined in the same object as their owning effect.

Every event callback is implemented as an fxlang program. An fxlang program is made up of many statements and blocks. A statement is a simple JSON string. Statements can be blocked together in an array.

Defined formally:

```
Program -> Leaf | Branch
Leaf -> JSON String
Branch -> JSON Array of Programs
```

This structure allows a program to be arbitrarily nested, similar to any other programming language. This sort of nesting is required for supporting conditionals and loops.

```json
{
  "program": [
    "statement 1",
    "statement 2",
    ["statement 3", ["statement 4", "statement 5"], "statement 6"],
    "statement 7"
  ]
}
```

Each line in an fxlang program must be a valid statement. The grammar is defined directly in the [abstract syntax tree representation](./battler/src/effect/fxlang/tree.rs).

The language is designed to be as simple to parse as possible. Minimal context is required to parse a string of tokens into a valid statement. The statement parser is implemented as a predictive recursive descent parser. It does not require backtracking because the next rule can always be inferred from the next token.

#### Values

First, values can be defined directly in the program as literals.

- Boolean literals are expressed as `true` or `false`.
- Number literals are expressed as integers or fractions. For example, `10`, `-25`, `1/10` are all valid number literals.
- String literals are a string of characters, optionally wrapped in single quotes. For example, `brn` and `'hello world'` are valid strings. Single quotes are required when there is whitespace or non-alphanumeric characters in the string. Single quotes are used to avoid needing to escape all string literals, since JSON strictly uses double quoted.

Values can also be defined dynamically using variables. All variables are prefixed with a `$`. For example, `$status`, `$target`, and `$mon_12` are all valid variables.

Values are strongly typed. Once a variable is assigned a value, it can only be assigned values of that same type.

Apart from the basic types, there are some more complex types:

- Lists are a sequential series of zero or more values (they do not need to be the same type). Lists can be defined inline using brackets: `[1, 'string', $mon]`.
- Objects are a generic key-value data structure. Values can be accessed by key using the member operator: `$object.first`, `$object.second`, `$object.nested.data`.

There are also types specific to the battle engine:

- Mons are references to Mons participating in a battle.
- Effects are references to generic battle effects, such as moves, abilities, statuses, and more.
- Active moves are references to moves being executed by a Mon on the current turn. Active moves are modifiable, so they are always separate from effects.

Battle-specific types also have a set of predefined immutable and mutable members, such as `$target.hp` or `$effect.id`.

##### Notes on Variables

1. All variables have program-wide scoping. In other words, variables are not scoped by block. A variable defined in an inner block is accessible in an outer block.
1. Invalid member accesses (such as accessing a member that does not exist) will error out the whole program. Some optional members will produce an "undefined" value that will fail on use rather, than fail on access.
1. Variables cannot be unassigned for the life of the program.
1. There are some variables that are defined before the program starts based on the callback's evaluation context, such as `$target`, `$move`, or `$effect_state`. This will be explored more in the evaluation section.

#### Function Call

The simplest statement is a function call. Functions are defined directly in the [battle engine](./battler/src/effect/fxlang/functions.rs), allowing callbacks to interact with the core battle engine. Zero or more arguments can be passed to the function. For example:

- `set_status: $target brn` - Calls the `set_status` function with two arguments. This applies the burn status to the target Mon.
- `random: 1 10` - Calls the `random` function with one argument. This generates a random number in the range `[1, 10)`.
- `chance: 2` - Calls the `chance` function with one argument. This returns a boolean indicating a 1/2 chance.
- `log_activate` - Calls the `log_activate` function with no arguments. Logs that the applying effect has activated, using the context of the callback.

#### Assignment

Another core statement is an assignment. The left-hand side of an assignment must be a mutable variable or mutable property of a variable, and the right-hand side is a value. For example, `$status = brn` - Assigns `'brn'` to the `$status` variable. This value can then be accessed later simply by using `$status`.

Note that some properties are strictly immutable. For example, `$mon.hp` is immutable. HP should be modified through other means (such as damaging the Mon).

#### Assigning a Return Value to a Variable

Function calls can optionally return a value. In our examples above `random: 1 10` should return a number while `chance: 2` should return a boolean. If you want to assign the return value of a function call to a variable, you must explicitly create a "function call value" using the `func_call` built-in.

- `$rand = func_call(random: 1 10)` - Assigns the result of the right-hand side function call to the `$rand` variable. This effectively stores a random number in the range `[1, 10)` in the variable `$rand`, to be accessed later without regenerating a number.
- `$chance = func_call(chance: 2)` - `$chance` is `true` 1/2 (50%) of the time.

#### Logging and String Formatting

A very important part of the battle engine is logging. The battle log represents the public output of a battle. Anything that should be visible to participants of a battle should be put in the output log. Consequentially, there are many functions defined specifically for logging in a common a way.

- `log: mustrecharge turn:2 reason:Unknown` - Adds the log `mustrecharge|turn:2|reason:Unknown` to the battle log.
- `log_activate` - Logs the "activate" event for the applying effect.
- `log_cant: Flinch` - Logs that the target of the effect's callback cannot move due to the "Flinch" effect.
- `log_status: Burn with_effect` - Logs that the target of the effect's callback has the "Burn" status, with the source effect added to the log. Note that `with_effect` here is a string literal interpreted by the `log_status` function to specialize behavior.

Note that nearly all of the logging functions such as the ones above use the context of the event callback to add information to the logs. For instance, `log_activate` on its own (with no arguments) will include the applying effect that the event callback is attached to.

Battle logs consist of a series of key-value properties. Logs often need to be generated dynamically based on the target of the effect (for instance, the Mon in the log must be based on the target of the effect). To support dynamic logs, fxlang has a string formatting built-in, `str`.

String formatting in fxlang looks extremely similar to string formatting in the Rust programming language. The first argument to `str` must be a string template. Each `{}` in the template is replaced with the next argument passed to the built-in. For example:

- `str('hello {}', $user)` - If `$user = world`, this statement generates the string `'hello world'`.
- `str('{} {} {}', $a, $b, $c)` - Generates a string containing all three variables.

It's now easy to piece together dynamic logs:

- `log: mustrecharge str('mon:{}', $target.position_details)` - Adds the log `mustrecharge|mon:Bulbasaur,player-1,1` to the battle log, which follows standard formatting.
- `log_activate: with_target with_source no_effect str('move:{}', $effect_state.source_effect.name)` - Logs the "activate" event with the source move name for the effect, the target Mon, and the source Mon. Note that `with_target`, `with_source`, and `no_effect` are all special strings interpreted by the `log_activate` function to specialize behavior.

#### Branching

A key requirement of dynamic battle effects is branching. For instance, `$chance = func_call(chance: 2)` emulates a coin flip, but how do we specialize behavior based on the result of this coin flip?

An "if" statement executes a following block based on a condition (a.k.a., boolean expression).

```json
["if func_call(chance: 2):", ["do_this", "and_this"], "else:", ["do_that"]]
```

In the above code, 50% of the time the block below the "if" statement will execute. The other 50% of the time, the block below the "else" statement will execute.

If statements can also be chained togehter with "else if" statements, which will run only a single branch of the group.

```json
[
  "$rand = func_call(random: 3)",
  "if $rand == 0:",
  ["$status = par"],
  "else if $rand == 1:",
  ["$status = frz"],
  "else:",
  ["$status = brn"],
  "set_status: $target $status"
]
```

The program above conditionally sets the `$status` variable based on a random number in the range `[0, 3)`. This is the exact definition of the secondary effect of the move "Tri Attack"!

#### Expressions

Notice that the if statements above allowed branching based on expressions that produced boolean results. The examples above are only a small subset of allowable expressions in fxlang.

Defined formally, an **expression** is a syntactic entity that always produces a value based on one or more values and operations.

The simplest expression is simply a value. `$mon.base_max_hp` is an expression that produces an integer result, and `$move.ohko` is an expression that produces a boolean result.

Expressions can be chained together using operators. The following list describes all operators:

1. `!a` - Negates `a` (`true` becomes `false`, and vice versa).
1. `a * b` - Multples `a` and `b`.
1. `a / b` - Divides `a` and `b`. Note that if both `a` and `b` are number literals, the result is coerced into a fraction at parsing time.
1. `a % b` - Returns the remainder of `a` divided by `b`.
1. `a + b` - Adds `a` and `b`.
1. `a - b` - Subtracts `a` and `b`.
1. `a < b` - Returns `true` if `a` is less than `b`; `false` otherwise.
1. `a <= b` - Returns `true` if `a` is less than or equal to `b`; `false` otherwise.
1. `a > b` - Returns `true` if `a` is greater than `b`; `false` otherwise.
1. `a >= b` - Returns `true` if `a` is greater than or equal to `b`; `false` otherwise.
1. `a has b` - Returns `true` if list `a` has an element equal to `b`; `false` otherwise.
1. `a hasany b` - Returns `true` if list `a` has any one of the elements in list `b`; `false` otherwise.
1. `a == b` - Returns `true` if `a` is equal to `b`; `false` otherwise.
1. `a != b` - Returns `true` if `a` is not equal to `b`; `false` otherwise.
1. `a and b` - Returns `true` if both `a` and `b` are `true`; `false` otherwise.
1. `a or b` - Returns `true` if either `a` or `b` are `true`; `false` otherwise.

##### Operator Precedence

In expressions where operators are arbitrarily written, certain groupings will be preferred to be evaluated before others. For example, `a + b * c` will unambiguously evaluate `b * c` before adding the result to `a`.

1. `!`
1. `*`, `/`, `%`
1. `+`, `-`
1. `<` `<=`, `>` `>=`, `has`, `hasany`
1. `==`, `!=`
1. `and`
1. `or`

Operator precedence can be manually broken by using parenthesis. For example, `(a + b) * c` will unambiguously evaluate `a + b` before multiplying the result with `c`.

##### Notes on Operators

1. The `and` and `or` operators implement short-circuiting. If the left-hand side of an `and` expression is false, the right-hand side will not be evaluated. If the left-hand side of an `or` expression is true, the right-hand side will not be evaluated.
1. Comparison operators, such as `<`, `>=`, or `==`, always produce a boolean value. Thus, it is invalid to chain comparisons like `a < b < c`, since this will effectively evaluate to `true < c`, which is an illegal statement. The correct form is `a < b and b < c`.
1. Numeric operations will pick the best type possible for the result. For example, a fraction multiplied by an integer will always produce a fraction. This should never be noticeable unless the numbers you are working with are approaching the reasonable limits of 32-bit integers (2147483647). If integer overflow occurs in either direction, the entire program will fail.
1. The negation (`!`) operator does allow for type coercion. For example, `!$a` is false for all defined variables (except `false` and `0`). This, along with short-circuiting, makes the negation operator perfect for verifying a variable is defined prior to using it: `if !$a or !$a.is_move:`.

#### Expression Values

It is often desired to use the result of an expression like a value, for function calls or variable asignment. Just like the `func_call` built-in wraps a function call statement into a value, the `expr` built-in wraps an expression into a value.

- `$damage = expr($damage / 2)` - Divides `$damage` by 2.
- `damage: expr($target.base_max_hp / 16)` - Applies damage to the target of the effet equal to 1/16 of their base maximum HP.
- `$something = func_call(max: expr($target.hp / 2), 1)` - Takes the maximum of `$target.hp / 2` and `1`, and assigns the result to `$something`.

#### Returning Values

Some callbacks must return a value to the battle engine. The easiest examples are damage callbacks, which determine the exact amount of damage to apply to Mon on an active move, or base power callbacks, which determine the base power to use for damage calculations.

A return statement signals that the program should terminate immediately and optionally send a value out of the program.

- `return` - Exits the program with no return value.
- `return 100` - Returns the number `100` from the program.
- `return expr($damage * 2)` - Returns twice the amount of damage previously stored.
- `return expr(func_call(random: 50 151) * $user.level / 100)` - Returns the damage calculation for the move "Psywave."

Return statements terminate the program immediately. Any following statements are ignored. This allows programs to conditionally exit at different times.

```json
[
  "if func_call(move_has_flag: $move thawing):",
  ["return"],
  "if func_call(chance: 1 5):",
  ["cure_status: $user", "return"],
  "log_cant: $this.name",
  ["return false"]
]
```

The above program has three different return statement:

1. Return nothing if the move has the "thawing" flag.
1. Return nothing if a 1/5 chance is met.
1. Return `false` otherwise.

This program runs before a user tries to use a move when they are frozen solid. For this event, `false` indicates the move cannot be used while "nothing" indicates no result (i.e., don't stop the move). The meaning of return values for different event types will be explored in the evaluation section.

#### Looping

A niche feature that can be nice to use is looping through a list of values and executing a program block for each value in the list.

A "for each" statement iterates over a list (in order). Each value is assigned to a named variable. The following block is executed once for each value in the list. For example:

```json
[
  "foreach $move_slot in $mon.move_slots:",
  [
    "$move = func_call(get_move: $move_slot.id)",
    "if func_call(move_has_flag: $move sound):",
    ["disable_move: $mon $move_slot.id"]
  ]
]
```

The above program loops through all of a Mon's move slots, disabling moves with the "sound" flag. This program implements the condition applied by the move "Throat Chop."

Below is another example for the move "Haze":

```json
["foreach $mon in func_call(all_active_mons):", ["clear_boosts: $mon"]]
```

### Parsing

Over the course of a battle, the callbacks for an effect may need to be evaluated numerous times. For example, many conditions apply themselves for multiple turns.

It would be inefficient to parse a program every time one of its event callbacks must be executed. Instead, all of the event callbacks for an effect are parsed at the same time at the effect's first appearance in the battle. The collection of parsed callbacks are then cached in the battle. The effect cache is implemented as an LRU (least-recently-used) cache that discards effects that were least-recently used when the cache size exceeds some threshold. Today, the maximum number of parsed callbacks in memory at a time per battle is `6 * 4 * 2 + 16`.

- 6 Mons per team.
- 4 moves per Mon.
- 2 teams per battle.
- Buffer of 16.

### Evaluation

fxlang programs are interpreted dynamically. JSON programs are parsed into a list of abstract syntax trees (one tree per statement), and each parsed statement is evaluated one after another.

#### Context

The first important concept about fxlang program evaluation is the evaluation context.

In the core battle engine, a `Context` object is a proxy object for getting references to battle data. For safety, Rust does not allow an object to be mutably borrowed multiple times. Rather than storing mutable references for as long as they are needed (so that mutable borrows will certianly overlap), references must be grabbed dynamically as they are needed. Context objects make this dynamic borrowing easy and safe to do.

Context objects are critical to the battle engine. Even something simple like calculating a Mon's attack stat cannot be done without a context. When we calculate a Mon's attack stat, we must also run a `ModifyAtk` event for effects active in the battle, since some effects can directly modify a Mon's attack stat. This requires access to the entire battle state, which can then cause mutations on different things across the battle and even the Mon itself. Thus, a simple stat calculation method requires the entire battle to get right (hopefully calculating the attack stat does not actually modify much globally, but the point still stands).

As a consequence, very few operations in the core battle engine are implemented as methods. Almost every important operation is implemented as a function that takes in a context. Contexts do act as "this" objects, since they can be scoped to things like Mons (`MonContext`), active moves (`ActiveMoveContext`), and effects (`EffectContext`).

Since event callbacks run in the context of a battle, the fxlang evaluator runs under some evaluation context that holds all of the battle state. Internally, during evaluation, the following state is kept on the context:

1. **Effect** - The effect whose event callback is being evaluated.
1. **Source Effect** (optional) - The effect that triggered this event callback.
1. **Target** (optional) - The target Mon of the source effect.
1. **Target Side** (optional) - The target side of the source effect.
1. **Source** (optional) - The source Mon that triggered the source effect.

In the code, this means we can evaluate event callbacks under the following contexts:

- `EffectContext` - The program runs under the context of an effect (which owns the event callback) and an optional source effect (that triggered the event).
- `ApplyingEffectContext` - The program runs under the context of an applying effect, which consists of an effect (which owns the event callback), an optional source effect (that triggered the event), the target Mon (that the source effect is being applied to), and an optional source Mon (that triggered the source effect).
- `SideEffectContext` - The program runs under the context of a side-applying effect, which consists of an effect (which owns the event callback), an optional source effect (that triggered the event), the target side (that the source effect is being applied to), and an optional source Mon (that triggered the source effect).
- `FieldEffectContext` - The program runs under the context of a field-applying effect, which consists of an effect (which owns the event callback), an optional source effect (that triggered the event), and an optional source Mon (that triggered the source effect).

#### Context Variables

At the start of each program, several context variables can be set based on the type of event being triggered. These variables can also be seen as input to the program.

The context variables to be set are defined directly by the type of event. For example:

- The `AddVolatile`, `SetStatus`, and `Duration` events set the `$target` (Mon) and `$effect` (effect) variables. They also set the `$source` (Mon) variable if one exists. These events run under the context of a generic applying effect.
- The `Hit`, `DamagingHit`, and `AfterMoveSecondaryEffects` events set the `$target` (Mon), `$move` (active move), and `$source` (Mon) variables. These events run under the context of an active move towards a target.
- The `MoveFailed`, `ModifyDamage`, and `UseMove` events set the `$user` (Mon) and `$move` (active move) variables. These events run under the context of an active move towards a user.

You can find all event definitions, including their context variable flags, in the [code](./battler/src/effect/fxlang/effect.rs).

It's important to remember the context under which a program is evaluating, as it determines which variables are directly available when the program starts.

Overall there are a handful of event callback categories:

1. **Applying Effect** - Callback that runs in the context of an applying effect on some Mon.
   - `$target` - The target Mon of the effect.
   - `$source` (optional) - The source Mon of the effect.
   - `$effect` - The source effect that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Side-Applying Effect** - Callback that runs in the context of an applying effect on some side.
   - `$side` - The target side of the effect.
   - `$source` (optional) - The source Mon of the effect.
   - `$effect` - The source effect that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Effect** - Callback that runs in the context of the effect itself.
   - `$target` - The target Mon of the effect.
   - `$source` (optional) - The source Mon of the effect.
   - `$source_effect` - The source effect that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **User-Focused Active Move** - Callback that runs in the context of an active move, focused on the user.
   - `$user` - The user of the move.
   - `$target` (optional) - The target Mon of the move.
   - `$move` - The active move that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Target-Focused Active Move** - Callback that runs in the context of an active move, focused on the target.
   - `$target` - The target of the move. Note that if there are multiple targets, the callback will run for each target implicitly.
   - `$source` The source (user) of the move.
   - `$move` - The active move that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Side-Focused Active Move** - Callback that runs in the context of an active move, focused on the target side.
   - `$side` - The target side of the move.
   - `$source` The source (user) of the move.
   - `$move` - The active move that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Field-Focused Active Move** - Callback that runs in the context of an active move, focused on hitting the field.
   - `$source` The source (user) of the move.
   - `$move` - The active move that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Individual Mon** - Callback that runs in the context of an individual Mon.
   - `$mon` - The target Mon.
   - `$this` - This effect that the event callback is running on.

##### Program Input

Event callbacks may also take in special input values, depending on the goal of the event. For example, an `AddVolatile` event callback provides a special `$volatile` input variable that contains the volatile status being added. A `ModifyDamage` event callback provides a `$damage` variable that can be modified and returned. A `TryBoost` event callback provides a `$boosts` variable to view and modify boosts that are going to be applied to a Mon.

##### Persistent State

Another special variable, `$effect_state`, is also defined before a program starts. The effect state is a generic object of key-value pairs that can be accessed and set however the effect sees fit. When a program terminates, the evaluator saves the `$effect_state` value to the battle engine. When a callback on the same effect runs again, the `$effect_state` will be set to the value from the battle engine.

You can think of `$effect_state` as a little persistent disk for an effect. It is mounted to the evaluation context of each event callback for an effect. For example, every time a callback of the "Toxic" status is run for a Mon, the same `$effect_state` is supplied, allowing the damage stacking part of the status to be easily implemented:

```json
{
  "callbacks": {
    "on_start": ["$effect_state.stage = 0"],
    "on_switch_in": ["$effect_state.stage = 0"],
    "on_residual": {
      "order": 9,
      "program": [
        "if $effect_state.stage < 15:",
        ["$effect_state.stage = $effect_state.stage + 1"],
        "damage: expr($target.base_max_hp / 16 * $effect_state.stage)"
      ]
    }
  }
}
```

In the above example, `$effect_state.stage` is initialized on start and switch in. At the end of every turn, the stage is incremented and used to damage the badly-poisoned Mon.

##### This Variable

Another special variable is the `$this` variable. `$this` is always set to the effect that the event callback originated from. For example, `remove_volatile: $user $this.id` will remove _this_ volatile effect from the `$user` Mon.

This variable is supplied largely as a convenience for accessing the ID and name of an effect.

#### Evaluating One Program

A program is evaluated one statement at a time, in accordance with the language definition described above. Function calls trigger core battle engine logic and are defined [here](./battler/src/effect/fxlang/functions.rs).

### Events

When a battle event is occurs, it triggers the associated event callbacks for active effects in the battle.

All supported events are implemented on the [`BattleEvent`](./battler/src/effect/fxlang/effect.rs) enum. Each event is described with its definition.

All methods that trigger an event are defined [here](./battler/src/battle/core_battle_effects.rs). Since Rust is a strongly-typed language, there is one method for each type of event trigger and expected output. This also makes the interface very easy to integrate with.

#### Triggering a Single Event (Mon or Active Move)

Sometimes an event needs to trigger only on a single effect. For example:

- A new volatile status has been added to a Mon and we must determine its duration using the `Duration` event.
- A Mon failed to use a move, so the `MoveFailed` event should trigger on the active move.

In this case, triggering the event callback is straightforward and evaluates a single event callback with direct input and output.

Some events exclusively trigger on a single effect, so it does not make sense to define it elsewhere. For example, the `UseMove` and `TryHit` events only trigger on an active move, so defining them on a status condition would do nothing (the callbacks would never get triggered).

#### Triggering a Global Event (Applying Effect)

Most often, an event needs to trigger globally and run all associated event callbacks. In this case, some special things happen during the evaluation:

1. All active effects for the scope of the event (i.e., target of the applying effect, which can be a Mon, side, or the whole battle field) are collected.
1. The active effects are filtered based on whether or not they have a callback for the triggering event.
1. The event callbacks are speed sorted (lower order first, then higher priority first, then lower sub-order first). This involves using RNG to break ties.
1. Callbacks are run under the evaluation context for the triggered event (i.e., a single Mon or an applying effect (which can also be an active move)).

Event callbacks are evaluated in order based on the order, priority, and sub-order defined in their definitions. Callbacks are evaluated as follows:

1. Pass the event callback input to the next callback.
1. Run the callback.
1. If the return value of the callback signals that the event should early exit, stop evaluating, and return the value of the callback as the result of the evaluation.
1. If the callback did not return any value, do nothing.
1. If the callback did return a value, modify the event callback input to contain the output of the previous callback's output.
1. Repeat for the next callback.
1. If all callbacks finish, return the first input variable as the result of the evaluation (this is the output of the last callback).

The above process effectively relays the input between callbacks and allows callbacks to prevent future callbacks from running. The most typical example of an early exit is a callback that returns `false`. For example, let's say that many active effects have a callback for the `BeforeMove` event. If the Mon flinched this turn, the Flinch `BeforeMove` callback can `return false` to signal that the Mon cannot move. In this case, there is no need to run any more callbacks. You can then define the order and priority of different `BeforeMove` callbacks to cause a Mon to flinch before or after other move-cancelling effects (such as being frozen solid or paralyzed).

Relaying values is extremely important for events like `ModifyDamage`, which continuously receives `$damage` as input and outputs a new number representing the modified damage.

For global events, returning no value (i.e., returning with just `return`, or just ending the program with no return) means that the callback was transparent and has no effect on the output of the event. If no value is returned, the relayed input is not overwritten. This effectively allows a `ModifyDamage` callback to only return a value when it's actually modified the damage value.

Scope matters a lot here for context variables. For example:

1. The `LockMove` event only runs under the scope of a single Mon, with no applying effect. It's easy to understand how `$effect` will be undefined but `$mon` will be defined.
1. The `AfterMove` event runs under the scope of an applying effect on the user of the move. Thus, the `$move` and `$user` variables will be defined.
1. The `DamagingHit` event runs under the scope of an applying effect on the target of the move. Thus, the `$move`, `$target`, and `$source` variables will all be defined. This event also provides `$damage` as an input variable.
1. The `SideStart` event runs under the scope of a side condition. Thus, only the `$side` variable will be defined. The `$source` variable may be defined, depending on if the side condition has a source Mon or not.

All contexts are documented on the [events themselves](./battler/src/effect/fxlang/effect.rs).

## Examples

### Simple Moves

#### Splash

Splash notoriously does nothing. The "But nothing happened!" log can be implemented by clients when the "Splash" move effect activates. Thus, our implementation is very easy:

```json
{
  "effect": {
    "callbacks": {
      "on_try_hit": ["log_activate"]
    }
  }
}
```

#### Super Fang

Super Fang deals damage equal to half of target's remaining HP. If this calculation yields no damage, the move will deal 1 damage.

```json
{
  "effect": {
    "callbacks": {
      "on_damage": ["return func_call(max: expr($target.hp / 2), 1)"]
    }
  }
}
```

#### Psywave

Psywave has its own custom damage calculation formula:

```json
{
  "effect": {
    "callbacks": {
      "on_damage": [
        "return expr(func_call(random: 50 151) * $source.level / 100)"
      ]
    }
  }
}
```

#### Jump Kick

If Jump Kick fails, the user keeps going, crashes, and loses 50% of its HP.

`$this.condition` is attached as the source of the damage to force the battle engine to log the reason for the crash damage (since the move condition is different from the active move itself).

```json
{
  "effect": {
    "callbacks": {
      "on_move_failed": [
        "damage: $user expr($user.base_max_hp / 2) $this.condition"
      ]
    }
  }
}
```

#### Magnitude

Magnitude randomly selects a magnitude, which determines the move's base power. The magnitude selected is also made known to the user. We can use a custom move message for the latter.

```json
{
  "effect": {
    "callbacks": {
      "on_use_move": [
        "$i = func_call(random: 100)",
        "if $i < 5:",
        ["$effect_state.magnitude = 4", "$move.base_power = 10"],
        "else if $i < 15:",
        ["$effect_state.magnitude = 5", "$move.base_power = 30"],
        "else if $i < 35:",
        ["$effect_state.magnitude = 6", "$move.base_power = 50"],
        "else if $i < 65:",
        ["$effect_state.magnitude = 7", "$move.base_power = 70"],
        "else if $i < 85:",
        ["$effect_state.magnitude = 8", "$move.base_power = 90"],
        "else if $i < 95:",
        ["$effect_state.magnitude = 9", "$move.base_power = 110"],
        "else:",
        ["$effect_state.magnitude = 10", "$move.base_power = 150"]
      ],
      "on_use_move_message": [
        "log_activate: str('magnitude:{}', $effect_state.magnitude)"
      ]
    }
  }
}
```

#### Tri Attack

Tri Attack has a 20% chance to either paralyze, freeze, or burn the target. This can actually be implemented as a callback on a _secondary effect_ of the move.

```json
{
  "secondary_effects": [
    {
      "chance": "20%",
      "effect": {
        "callbacks": {
          "on_hit": [
            "$rand = func_call(random: 3)",
            "if $rand == 0:",
            ["$status = par"],
            "else if $rand == 1:",
            ["$status = frz"],
            "else:",
            ["$status = brn"],
            "set_status: $target $status"
          ]
        }
      }
    }
  ]
}
```

### Status Conditions

#### Burn

Burn applies residual damage and also halves damage dealt by physical moves.

```json
{
  "condition": {
    "callbacks": {
      "on_start": [
        "if $source_effect.is_ability:",
        ["log_status: $this.name with_source_effect"],
        "else:",
        ["log_status: $this.name"]
      ],
      "on_residual": {
        "order": 10,
        "program": ["damage: expr($target.base_max_hp / 16)"]
      },
      "on_modify_damage": {
        "order": 1,
        "program": [
          "if $move.category == physical and !func_call(has_ability: $user guts) and $move.id != facade:",
          ["return expr($damage / 2)"]
        ]
      }
    }
  }
}
```

#### Freeze

Freeze completley immobilizes the target until it is thawed at the beginning of a turn (20% chance). A Mon can also be thawed by a Fire type move or a thawing move (either by the user or target).

```json
{
  "condition": {
    "callbacks": {
      "on_start": [
        "if $source_effect.is_ability:",
        ["log_status: $this.name with_source_effect"],
        "else:",
        ["log_status: $this.name"]
      ],
      "on_before_move": {
        "priority": 10,
        "program": [
          "if func_call(move_has_flag: $move thawing):",
          ["return"],
          "if func_call(chance: 1 5):",
          ["cure_status: $user", "return"],
          "log_cant: $this.name",
          ["return false"]
        ]
      },
      "on_use_move": [
        "if func_call(move_has_flag: $move thawing):",
        ["cure_status: $user use_source log_effect"]
      ],
      "on_after_move_secondary_effects": [
        "if $move.thaws_target:",
        ["cure_status: $target use_source"]
      ],
      "on_damaging_hit": [
        "if $move.type == fire and $move.category != status:",
        ["cure_status: $target use_source"]
      ]
    }
  }
}
```

### Volatile Statuses

#### Confusion

A Mon is confused for a set amount of turns. On each turn it is confused, there is a 33% chance the Mon will hit itself in confusion.

We use a custom time state variable because confusion does not wear off at the end of a turn. Instead, a Mon snaps out of confusion right before it uses a move.

```json
{
  "condition": {
    "callbacks": {
      "on_start": [
        "if $source_effect.id == lockedmove:",
        ["log_start: fatigue"],
        "else:",
        ["log_start"],
        "$effect_state.time = func_call(random: 2 6)"
      ],
      "on_end": ["log_end"],
      "on_before_move": {
        "priority": 3,
        "program": [
          "$effect_state.time = $effect_state.time - 1",
          "if $effect_state.time == 0:",
          ["remove_volatile: $user $this.id", "return"],
          "log_activate: with_target",
          "if !func_call(chance: 33 100):",
          ["return"],
          "$damage = func_call(calculate_confusion_damage: $user 40)",
          "damage: no_source $user $damage $this",
          "return false"
        ]
      }
    }
  }
}
```

#### Locked Move

Moves like Thrash or Outrage lock the user into a move for 2-3 turns and confuse the target from fatigue afterwards. Notice that we use the `AfterMove` event to end the volatile status earlier.

```json
{
  "condition": {
    "callbacks": {
      "on_duration": ["return func_call(random: 2 4)"],
      "on_start": ["$effect_state.move = $source_effect.id"],
      "on_after_move": [
        "if $user.move_this_turn_failed:",
        ["remove_volatile: $user $this.id no_events"],
        "else if $effect_state.duration == 1:",
        ["remove_volatile: $user $this.id"]
      ],
      "on_move_aborted": ["remove_volatile: $user $this.id no_events"],
      "on_end": ["add_volatile: $target confusion"],
      "on_lock_move": ["return $effect_state.move"]
    }
  }
}
```

### Side Conditions

#### Mist

Mist protects all Mons on the user's side from stat drops from opposing Mons.

```json
{
  "hit_effect": {
    "side_condition": "mist"
  },
  "condition": {
    "duration": 5,
    "callbacks": {
      "on_try_boost": [
        "if $effect.infiltrates and !func_call(is_ally: $target $source):",
        ["return"],
        "if !$source or $source == $target:",
        ["return"],
        "$activated = false",
        "foreach $stat in func_call(boostable_stats):",
        [
          "if func_call(get_boost: $boosts $stat) < 0:",
          [
            "$boosts = func_call(set_boost: $boosts $stat 0)",
            "$activated = true"
          ]
        ],
        "if $activated:",
        ["log_activate: str('mon:{}', $target.position_details)"],
        "return $boosts"
      ],
      "on_side_start": ["log_side_start"],
      "on_side_end": ["log_side_end"]
    }
  }
}
```

### Complex Examples

#### Fly

Fly is a two-turn move (in other words, it has a charging turn). Since many other moves have a very similar two-turn move structure, we define a two-turn move has its own type of volatile status.

A Mon with the "Two Turn Move" volatile status gets a volatile condition for the move it is charging. Some additional logic is also added for preventing the move from animating (as it has not been used yet) and for running an event associated with the user preparing to hit its targets.

```json
{
  "name": "Two Turn Move",
  "condition_type": "Built-in",
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": [
        "$effect_state.move = $source_effect.id",
        "add_volatile: $target $source_effect.id",
        "do_not_animate_last_move",
        "# Still run events associated with the user preparing to hit the target, since they are locked into this move.",
        "run_event: PrepareHit"
      ],
      "on_set_last_move": ["if $effect_state.duration > 1:", ["return false"]],
      "on_deduct_pp": {
        "order": 999,
        "program": [
          "# Run last, to ensure no PP is deducted while charging.",
          "if $effect_state.duration > 1:",
          ["return 0"]
        ]
      },
      "on_lock_move": ["return $effect_state.move"],
      "on_move_aborted": ["remove_volatile: $target $effect_state.move"],
      "on_end": ["remove_volatile: $target $effect_state.move"]
    }
  }
}
```

Then, we can define Fly as a move that applies this volatile status:

```json
{
  "effect": {
    "callbacks": {
      "on_try_use_move": [
        "if func_call(remove_volatile: $user $this.id):",
        ["return"],
        "log_prepare_move",
        "if !func_call(run_event: ChargeMove):",
        ["return"],
        "add_volatile: $user twoturnmove",
        "return stop"
      ]
    }
  }
}
```

The first check in the above callback is the most important: if the Mon has the "fly" volatile status, that means it received it from the "twoturnmove" volatile status that it received last turn, which means it successfully executed its charge turn. The `return` allows the move to be used as it normally would. You can imagine other checks here that would skip the charge turn (like Solar Beam being used in harsh sunlight weather).

Finally, a Mon in the "flying" state has some special invulnerability and damage rules:

```json
{
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_invulnerability": [
        "if [gust, twister, skyuppercut, thunder, hurricane, smackdown, thousandarrows] has $move.id:",
        ["return"],
        "return false"
      ],
      "on_source_modify_damage": [
        "if [gust, twister] has $move.id:",
        ["return expr($damage * 2)"]
      ]
    }
  }
}
```

The `Invulnerability` callback grants the Mon using fly invulnerability from most moves except for an exception list. The `SourceModifyDamage` callback runs when the Mon is the target of a Mon modify damaging against it (in other words, this Mon is the source of the `ModifyDamage` event). The moves "Gust" and "Twister" are powered up against Mons in the air.

#### Metronome

Metronome executes a random move. This is actually simpler than you might think, and only requires fetching all potential moves and sampling one out using RNG (thus producing a consistent, replayable result).

```json
{
  "effect": {
    "callbacks": {
      "on_hit": [
        "$moves = func_call(get_all_moves: without_flag:nometronome)",
        "$random_move = func_call(sample: $moves)",
        "if !$random_move:",
        ["return false"],
        "use_move: $source $random_move.id"
      ]
    }
  }
}
```

#### Bide

Bide stores up damage applied to the user for several turns. On the third turn, the user attacks the Mon that last damaged it with twice the stored damage. When Bide is first used, it applies the "bide" volatile status to the user, which implements all damage-storing event callbacks.

To unleash the damage, Bide actually uses a custom version of itself that applies the stored damage to the target. The custom version of Bide is stored in the conditions `local_data`, which is a place where custom data can be defined for use by event callbacks.

The benefit here is that the modified move can be written statically in the condition code, rather than dynamically inside of the event callback (pretty much every field would need to be overwritten). Furthermore, this customized version of Bide can actually have its _own_ event callbacks. In this case, the `TryUseMove` callback fails the move if no damage would be applied. And by setting `no_random_target`, the move also fails if the Bide volatile condition did not have any target for the move. Thus, the two ways of failing the move are covered directly in the core battle engine rather than in the dynamic event callback code.

```json
{
  "hit_effect": { "volatile_status": "bide" },
  "condition": {
    "duration": 3,
    "callbacks": {
      "on_start": ["$effect_state.total_damage = 0", "log_start"],
      "on_restart": ["return true"],
      "on_lock_move": ["return $this.id"],
      "on_damage_received": [
        "if func_call(is_defined: $source) and $source != $target:",
        ["$effect_state.last_damage_source = $source"],
        "$effect_state.total_damage = $effect_state.total_damage + $damage"
      ],
      "on_before_move": [
        "# This callback runs when the user is storing energy.",
        "if $effect_state.duration > 1:",
        ["log_activate: str('mon:{}', $user.position_details)", "return"],
        "# Bide is ending this turn, so this use of the move unleashes the energy.",
        "log_end",
        "$target = $effect_state.last_damage_source",
        "# Create a new active move that deals the damage to the target, and use it directly.",
        "$move = func_call(new_active_move_from_local_data: $this.id)",
        "$move.damage = expr($effect_state.total_damage * 2)",
        "# Remove this volatile effect before using the new move, or else this callback gets triggered endlessly.",
        "remove_volatile: $user $this.id",
        "use_active_move: $user $move $target",
        "# Since we used the local Bide, we can exit this move early.",
        "return false"
      ],
      "on_move_aborted": ["remove_volatile: $user $this.id"]
    },
    "local_data": {
      "moves": {
        "bide": {
          "name": "Bide",
          "category": "Physical",
          "primary_type": "Normal",
          "accuracy": "exempt",
          "priority": 1,
          "target": "Scripted",
          "flags": ["Contact", "Protect"],
          "ignore_immunity": true,
          "no_random_target": true,
          "effect": {
            "callbacks": {
              "on_try_use_move": [
                "# Fail if no direct damage was received.",
                "if $move.damage == 0:",
                ["return false"]
              ]
            }
          }
        }
      }
    }
  }
}
```

#### Substitute

Substitute is a bit of an exception, since it does things unlike any other move in the battle engine. Substitute takes a quarter of the user's HP and applies it to a substitute. That substitute has the same amount of HP and will absorb all hit effects (damage, stat boosts/drops, statuses, volatiles). Once the substitute runs out of HP, it will disappear, and the Mon will be hittable again.

Since Substitute is so niche, it actually gets its own event, `TryPrimaryHit`, that allows a move to completely override how a move is applied to a Mon. If a callback for this event returns 0, the core battle engine assumes a substitute was hit, and the target becomes exempt from any other effect of the move.

Here is the code in all of its glory:

```json
{
  "hit_effect": {
    "volatile_status": "substitute"
  },
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "if func_call(has_volatile: $source substitute) or $source.hp <= $source.max_hp / 4 or $source.max_hp == 1:",
        ["log_fail: $source", "return stop"]
      ],
      "on_hit": ["direct_damage: expr($target.max_hp / 4)"]
    }
  },
  "condition": {
    "callbacks": {
      "on_start": [
        "log_start",
        "$effect_state.hp = func_call(floor: expr($target.max_hp / 4))",
        "if func_call(has_volatile: $target partiallytrapped):",
        ["remove_volatile: $target partiallytrapped"]
      ],
      "on_try_primary_hit": [
        "# Some moves can hit through substitute.",
        "if $target == $source or func_call(move_has_flag: $move bypasssubstitute) or $move.infiltrates:",
        ["return"],
        "# Calculate and apply damage.",
        "$damage = func_call(calculate_damage: $target)",
        "if func_call(is_boolean: $damage) and !$damage:",
        ["log_fail: $source", "do_not_animate_last_move", "return false"],
        "if $damage > $effect_state.hp:",
        ["$damage = $effect_state.hp"],
        "$effect_state.hp = $effect_state.hp - $damage",
        "# Break the substitute when HP falls to 0.",
        "if $effect_state.hp == 0:",
        [
          "if $move.ohko:",
          ["log_ohko: $target"],
          "remove_volatile: $target substitute"
        ],
        "else:",
        ["log_activate: damage"],
        "# Some move effects still apply.",
        "apply_recoil_damage: $damage",
        "apply_drain: $source $target $damage",
        "run_event_on_move: AfterSubstituteDamage",
        "run_event: AfterSubstituteDamage",
        "return 0"
      ],
      "on_end": ["log_end: $this.name"]
    }
  }
}
```

Let's walk through some of it:

1. When Substitute is used, the user must have enough HP for the substitute.
1. The user receives the "substitute" volatile status, which is defined directly on the move.
1. Moves that bypass the substitute cause the `TryPrimaryHit` callback to return no value, which causes the move to execute as normal.
1. Otherwise, some fundamental things that normally happen in the core battle engine occur:
   1. Damage is calculated using the normal damage calculation formula.
   1. The move fails if the damage calculation fails.
   1. The substitute breaks if the user deals more damage than it has HP.
   1. If the substitute survives, the substitute effect activates in the log.
   1. Some core move effects on the user still apply, like HP drain and recoil damage.
   1. There is a special event for substitute damage occurring, since there is a distinction between regular damage and substitute damage.
