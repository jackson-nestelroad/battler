# fxlang

**fxlang** is the JSON-based interpreted language used in **battler** for all battle effects.

## Motivation

Pokémon battles are complex. Many things can impact many parts of a battle: moves, abilities, items, statuses, volatile statuses (which can stack), weather, field effects, and more. Furthermore, many things can be impacted by these different effects, from calculations (e.g., damage, type effectiveness, accuracy, etc.) to other effects themselves (such as those on a field, side, or individual Mon). This high complexity makes supporting 900+ moves and 180+ abilities practically impossible in the core logic of a battle engine.

Thus, there is a need for making battle effects easy to program for different battle events and conditions.

## Definitions

An **effect** is anything that impacts some part of a battle, such as a move, ability, item, status, weather, field effect, and more.

An **event** is something that happens in a battle that triggers effects to activate. Some easy examples are when a move is used, when a Mon takes damage, or when a Mon switches in.

An **event callback** is logic that runs for an individual effect on the firing of some event. One effect can have multiple event callbacks to run logic on different events.

Our goal is to allow a multitude of _effects_ to define a set of _event callbacks_ that will be triggered by battle _events_.

## Potential Solutions

We are looking for a solution that

1. is compatible with the Rust language;
1. is easy to extend for new behavior; and
1. is relatively straightforward to use, even for complex effects.

An obvious solution would be to just write different event callbacks for each effect directly in Rust. However, this solution is inflexible and is not straightforward to use, because new effects must be written directly in Rust and built directly into the binary. Furthermore, the battle library represents data in JSON, so effect callbacks and data would be completely separate.

Another solution is to create a large set of data fields that the battle library can understand to run the effect correctly. This solution is simple for most effects (for example, most effects deal damage, and most secondary effects are simple stat changes or status effects). Unfortunately, it is practically impossible to generalize all 1000+ battle effects into a set of scalar fields without many strange outliers. For example, random values cannot easily be represented in this format. Many effects also check preconditions before deciding to apply the effect at all. Various parts of the battle can be interrupted or short-circuited due to complex interactions. All in all, complex moves will always require some custom programming.

The solution we opt for is an interpreted language that can be expressed directly in JSON for different event callbacks. An interpreted language can be compatible with any programming language, extended for new behavior, and developed by external users with less knowledge of the internals of the battle engine itself (i.e., the interpreted language can hide away some complexities).

## Design

**fxlang** (short for "effects language") is a JSON-based interpreted language for writing battle effect event callbacks. When an event occurs in battle, the battle engine will gather the active effects in the battle and run any associated callbacks for the event.

### Language

Like all other data for the battle engine, callbacks are defined directly in JSON, allowing callbacks to be defined in the same object as their owning effect.

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
- String literals are a string of characters, optionally wrapped in single quotes. For example, `brn` and `'hello world'` are valid strings. Single quotes are required when there is whitespace or non-alphanumeric characters in the string. Single quotes are used to avoid needing to escape all string literals, since JSON strictly uses double quotes.

Values can also be defined dynamically using variables. All variables are prefixed with a `$`. For example, `$status`, `$target`, and `$mon_12` are all valid variables.

Values are strongly typed. Once a variable is assigned a value, it can only be assigned values of that same type.

Apart from the basic types, there are some more complex types:

- Lists are a sequential series of zero or more values (they do not need to be the same type). Lists can be defined inline using brackets: `[1, 'string', $mon]`.
- Objects are a generic key-value data structure. Values can be accessed by key using the member operator: `$object.first`, `$object.second`, `$object.nested.data`.

There are also types specific to the battle engine:

- Mons are references to Mons participating in a battle.
- Effects are references to generic battle effects, such as moves, abilities, statuses, and more.
- Active moves are references to moves being executed by a Mon on the current turn. Active moves are modifiable, so they are always separate from effects.
- Some other types exist for battle-specific functions, such as a player in the battle, a side of the battle, or the entire field itself.
- Many other battle-specific value types exist, such as types, move categories, stats, stat tables, and more!

Battle-specific types also have a set of predefined immutable and mutable members, such as `$target.hp` or `$effect.id`.

##### Notes on Variables

1. All variables have program-wide scoping. In other words, variables are not scoped by block. A variable defined in an inner block is accessible in an outer block.
1. Invalid member accesses (such as accessing a member that does not exist) will error out the whole program. Some optional members will produce an "undefined" value that will fail on use rather than fail on access.
1. Variables cannot be unassigned for the life of the program.
1. There are some variables that are defined before the program starts based on the callback's evaluation context, such as `$target`, `$move`, or `$effect_state`. This will be explored more in the evaluation section.

#### Function Call

The simplest statement is a function call. Functions are defined directly in the [battle engine](./battler/src/effect/fxlang/functions.rs), allowing callbacks to interact with the core battle engine. Zero or more arguments can be passed to the function. For example:

- `set_status: $target brn` - Calls the `set_status` function with two arguments. This applies the burn status to the target Mon.
- `random: 1 10` - Calls the `random` function with one argument. This generates a random number in the range `[1, 10)`.
- `chance: 2` - Calls the `chance` function with one argument. This returns a boolean indicating a 1/2 chance.
- `log_activate` - Calls the `log_activate` function with no arguments. Logs that the applying effect has activated, using the context of the callback.

#### Assignment

Another core statement is an assignment. The left-hand side of an assignment must be a mutable variable or mutable property of a variable, and the right-hand side must be a value. For example, `$status = brn` - Assigns `'brn'` (a string) to the `$status` variable. This value can then be accessed later simply by using `$status`.

Note that some properties are strictly immutable. For example, `$mon.hp` is immutable. HP should be modified through other means (such as damaging the Mon with the `damage` function).

#### Assigning a Return Value to a Variable

Function calls can optionally return a value. In our examples above `random: 1 10` should return a number while `chance: 2` should return a boolean value. If you want to assign the return value of a function call to a variable, you must explicitly create a "function call value" using the `func_call` built-in.

- `$rand = func_call(random: 1 10)` - Assigns the result of the right-hand side function call to the `$rand` variable. This effectively stores a random number in the range `[1, 10)` in the variable `$rand`, to be accessed later without regenerating a number.
- `$chance = func_call(chance: 2)` - `$chance` is `true` 1/2 (50%) of the time.

#### Logging and String Formatting

A very important part of the battle engine is logging. The battle log represents the public output of a battle. Anything that should be visible to participants of a battle should be put in the output log. Consequentially, there are many functions defined specifically for logging in a common a way.

- `log: helloworld 'turn:2' 'reason:Unknown'` - Adds the log `helloworld|turn:2|reason:Unknown` to the battle log.
- `log_status: 'Burn'` - Logs that the target of the effect's callback has the "Burn" status.
- `log_activate: with_target` - Logs the "activate" event for the applying effect with the target of the effect added to the log. Note that `with_target` here is a string literal interpreted by the `log_activate` function to specialize behavior.

Note that nearly all the logging functions such as the ones above use the context of the event callback to add information to the logs. For instance, `log_activate` on its own (with no arguments) will include the applying effect that the event callback is attached to.

Battle logs consist of a series of key-value properties. Logs often need to be generated dynamically based on the target of the effect (for instance, the Mon in the log must be based on the target of the effect). To support dynamic logs, fxlang has a string formatting built-in, `str`.

String formatting in fxlang looks extremely similar to string formatting in the Rust programming language. The first argument to `str` must be a string template. Each `{}` in the template is replaced with the next argument passed to the built-in. For example:

- `str('hello {}', $user)` - If `$user = world`, this statement generates the string `'hello world'`.
- `str('{} {} {}', $a, $b, $c)` - Generates a string containing all three variables.

It's now easy to piece together dynamic logs:

- `log_start: str('disabledmove:{}', $target.last_move.name)` - Adds the log `start|mon:Bulbasaur,player-1,1|move:Disable|disabledmove:Tackle` to the battle log. This is the log for the start of the "Disable" move effect.
- `log_activate: with_target use_source str('newmove:{}', $last_move.name)"` - Logs the "activate" event with the target of the effect, using the source Mon as the target, along with a custom "newmove" entry. This is the log for the move "Sketch" when it copies a move.

As noted briefly above, logging functions, especially `log_activate`, have several input tags that have special meaning for customizing logs consistently. In fact, these tags can affect a wider array of battle functions. These effect modification tags are discussed later.

#### Branching

A key requirement of dynamic battle effects is branching. For instance, `$chance = func_call(chance: 2)` emulates a coin flip, but how do we specialize behavior based on the result of this coin flip?

An "if" statement executes a following block based on a condition (a.k.a., boolean expression).

```json
["if func_call(chance: 2):", ["do_this", "and_this"], "else:", ["do_that"]]
```

In the above code, 50% of the time the block below the "if" statement will execute. The other 50% of the time, the block below the "else" statement will execute.

If statements can also be chained together with "else if" statements, which will run only a single branch of the group.

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
1. `+a` - Makes `a` into a signed number.
1. `a ^ b` - Exponentiates `a` by `b` (i.e., `a` to the power of `b`)
1. `a * b` - Multiplies `a` and `b`.
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

1. `!`, `+` (unary)
1. `^`
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
1. The negation (`!`) operator does allow for type coercion. For example, `!$a` is false for all defined variables (except `false` and `0`). This, along with short-circuiting, makes the negation operator perfect for verifying a variable is defined prior to using it: `if !$a or !$a.is_move:`. You can also check for undefined variables explicitly by using `$a.is_undefined`.

#### Expression Values

It is often desired to use the result of an expression like a value, for function calls or variable assignment. Just like the `func_call` built-in wraps a function call statement into a value, the `expr` built-in wraps an expression into a value.

- `$damage = $damage / 2` - Divides `$damage` by 2.
- `damage: $target expr($target.base_max_hp / 16)` - Applies damage to the target of the effect equal to 1/16 of their base maximum HP.
- `$something = func_call(max: expr($target.hp / 2), 1)` - Takes the maximum of `$target.hp / 2` and `1`, and assigns the result to `$something`.

#### Returning Values

Some callbacks must return a value to the battle engine. The easiest examples are damage callbacks, which determine the exact amount of damage to apply to Mon on an active move, or base power callbacks, which determine the base power to use for damage calculations.

A return statement signals that the program should terminate immediately and optionally send a value out of the program.

- `return` - Exits the program with no return value.
- `return 100` - Returns the number `100` from the program.
- `return $damage * 2` - Returns twice the amount of damage previously stored.
- `return func_call(random: 50 151) * $user.level / 100` - Returns the damage calculation for the move "Psywave."

Return statements terminate the program immediately. Any following statements are ignored. This allows programs to conditionally exit at different times.

```json
[
  "if func_call(move_has_flag: $move thawing):",
  ["return"],
  "if func_call(chance: 1 5):",
  ["cure_status: $user no_effect", "return"],
  "log_cant",
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
    "if func_call(move_has_flag: $move_slot.id sound):",
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

It would be inefficient to parse a program every time one of its event callbacks must be executed. Instead, all the event callbacks for an effect are parsed at the same time at the effect's first appearance in the battle. The collection of parsed callbacks are then cached in the battle. The effect cache is implemented as an LRU (least-recently-used) cache that discards effects that were least-recently used when the cache size exceeds some threshold.

### Evaluation

fxlang programs are interpreted dynamically. JSON programs are parsed into a list of abstract syntax trees (one tree per statement), and each parsed statement is evaluated one after another.

#### Context

The first important concept about fxlang program evaluation is the evaluation context.

In the core battle engine, a `Context` object is a proxy object for getting references to battle data. For safety, Rust does not allow an object to be mutably borrowed multiple times. Rather than storing mutable references for as long as they are needed (so that mutable borrows will certainly overlap), references must be grabbed dynamically as they are needed. Context objects make this dynamic borrowing easy and safe to do.

Context objects are critical to the battle engine. Even something simple like calculating a Mon's attack stat cannot be done without a context. When we calculate a Mon's attack stat, we must also run a `ModifyAtk` event for effects active in the battle, since some effects can directly modify a Mon's attack stat. This requires access to the entire battle state, which can then cause mutations on different things across the battle and even the Mon itself. Thus, a simple stat calculation method requires the entire battle to get right (hopefully calculating the attack stat does not actually modify much globally, but the point still stands).

As a consequence, very few operations in the core battle engine are implemented as methods. Almost every important operation is implemented as a function that takes in a context. Contexts do act as "this" objects, since they can be scoped to things like Mons (`MonContext`), active moves (`ActiveMoveContext`), and effects (`EffectContext`).

Since event callbacks run in the context of a battle, the fxlang evaluator runs under some evaluation context that holds all the battle state. Internally, during evaluation, the following state is kept on the context:

1. **Effect** - The effect whose event callback is being evaluated.
1. **Source Effect** (optional) - The effect that triggered this event callback.
1. **Target** (optional) - The target Mon of the source effect.
1. **Target Side** (optional) - The target side of the source effect.
1. **Source** (optional) - The source Mon that triggered the source effect.

In the code, this means we can evaluate event callbacks under the following contexts:

- `EffectContext` - The program runs under the context of an effect (which owns the event callback) and an optional source effect (that triggered the event).
- `ApplyingEffectContext` - The program runs under the context of an applying effect, which consists of an effect (which owns the event callback), an optional source effect (that triggered the event), the target Mon (that the source effect is being applied to), and an optional source Mon (that triggered the source effect).
- `PlayerEffectContext` - The program runs under the context of a player-applying effect, which consists of an effect (which owns the event callback), an optional source effect (that triggered the event), the target player (that the source effect is being applied to), and an optional source Mon (that triggered the source effect).
- `SideEffectContext` - The program runs under the context of a side-applying effect, which consists of an effect (which owns the event callback), an optional source effect (that triggered the event), the target side (that the source effect is being applied to), and an optional source Mon (that triggered the source effect).
- `FieldEffectContext` - The program runs under the context of a field-applying effect, which consists of an effect (which owns the event callback), an optional source effect (that triggered the event), and an optional source Mon (that triggered the source effect).

#### Context Variables

At the start of each program, several context variables can be set based on the type of event being triggered. These variables can also be seen as input to the program.

The context variables to be set are defined directly by the type of event. For example:

- The `AddVolatile`, `SetStatus`, and `Duration` events set the `$target` (Mon) and `$effect` (effect) variables. They also set the `$source` (Mon) variable if one exists. These events run under the context of a generic applying effect.
- The `Hit`, `DamagingHit`, and `AfterMoveSecondaryEffects` events set the `$target` (Mon), `$move` (active move), and `$source` (Mon) variables. These events run under the context of an active move towards a target (i.e., the target is the focus of the event).
- The `MoveFailed`, `ModifyDamage`, and `UseMove` events set the `$user` (Mon) and `$move` (active move) variables. They also set the `$target` (Mon) variable if one exists. These events run under the context of an active move towards a user (i.e., the user is the focus of the event).

You can find all event definitions, including their context variable flags, in the [code](./battler/src/effect/fxlang/effect.rs).

It's important to remember the context under which a program is evaluating, as it determines which variables are directly available when the program starts.

##### This Variable

Another special variable is the `$this` variable. `$this` is always set to the effect that the event callback originated from. For example, `remove_volatile: $user $this.id` will remove _this_ volatile status from the `$user` Mon.

This variable is supplied largely as a convenience for accessing the ID and name of an effect.

##### Variables Defined per Callback Category

Overall there are a handful of event callback categories:

1. **Applying Effect** - Callback that runs in the context of an applying effect on some Mon.
   - `$target` - The target Mon of the effect.
   - `$source` (optional) - The source Mon of the effect.
   - `$effect` - The source effect that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Player-Applying Effect** - Callback that runs in the context of an applying effect on some side.
   - `$player` - The target player of the effect.
   - `$source` (optional) - The source Mon of the effect.
   - `$effect` - The source effect that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Side-Applying Effect** - Callback that runs in the context of an applying effect on some side.
   - `$side` - The target side of the effect.
   - `$source` (optional) - The source Mon of the effect.
   - `$effect` - The source effect that is triggering the callback.
   - `$this` - This effect that the event callback is running on.
1. **Field-Applying Effect** - Callback that runs in the context of an applying effect on some side.
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

Another special variable, `$effect_state`, is also defined before a program starts. The effect state is a generic object of key-value pairs that can be accessed and set however the effect sees fit. Any mutation to the effect state is updated in the battle engine immediately, so mutations to the state from other callbacks will be visible immediately.

You can think of `$effect_state` as a little persistent disk for an effect. It is mounted to the evaluation context of each event callback for an effect. For example, every time a callback of the "Toxic" status is run for a Mon, the same `$effect_state` is supplied, allowing the damage stacking part of the status to be easily implemented:

```json
{
  "callbacks": {
    "on_start": ["$effect_state.stage = 0", "log_status: $this.name"],
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

#### Evaluating One Program

A program is evaluated one statement at a time, in accordance with the language definition described above. Function calls trigger core battle engine logic and are defined [here](./battler/src/effect/fxlang/functions.rs).

### Events

When a battle event is occurs, it triggers the associated event callbacks for active effects in the battle.

All supported events are implemented on the [`BattleEvent`](./battler/src/effect/fxlang/effect.rs) enum. Each event is described with its definition.

All methods that trigger an event are defined [here](./battler/src/battle/core_battle_effects.rs). Since Rust is a strongly-typed language, there is one method for each type of event trigger and expected output. This also makes the interface very easy to integrate with.

#### Triggering a Single Event

Sometimes an event needs to trigger only on a single effect. For example:

- A new volatile status has been added to a Mon, and we must determine its duration using the `Duration` event.
- A Mon failed to use a move, so the `MoveFailed` event should trigger on the active move.

In this case, triggering the event callback is straightforward and evaluates a single event callback with direct input and output.

Some events exclusively trigger on a single effect, so it does not make sense to define it elsewhere. For example, the `ModifyTarget` and `TryUseMove` events only trigger on an active move, so defining them on a status condition would do nothing (the callbacks would never get triggered).

#### Triggering a Global Event (Applying Effect)

Most often, an event needs to trigger globally and run all associated event callbacks. In this case, some special things happen during the evaluation:

1. All active effects for the scope of the event (i.e., target of the applying effect, which can be a Mon, side, or the whole battlefield) are collected.
1. The active effects are filtered based on if they have a callback for the triggering event.
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

## Creating Effects with fxlang (with Examples)

We now know how to write fxlang callbacks and how they execute based on different battle events. This section covers general guidance on how to write effects, both simple and complex. Each section contains several examples.

For any generic effect, we want to do the following:

1. When some event occurs, trigger a callback (fxlang program).
1. In that effect callback, check if some condition is satisfied (branching).
1. If that condition is met, trigger the effect.
   1. The effect may cause the overall event and/or source effect to fail (return a value signaling failure).
   1. The effect may modify some calculation (return a modified value).
   1. The effect may trigger some additional effect (function call).

### Choosing Events

First, it's important to choose which events to hook callbacks into. The best thing you can do here is **identify similar effects and use their callbacks as a starting point**. Otherwise, the general order in which events trigger at different parts of a battle are [documented for convenience](./events.md).

### Calculation Modifiers

Many effects modify some calculation in some way. The damage and base power calculations for moves can be altered by abilities, items, conditions, and even moves themselves.

These are the simplest types of effects to write, as we only need to write event callbacks that modify the calculation in the appropriate event.

#### Examples

##### Move: Super Fang

Super Fang has a custom damage calculation, where it exactly cuts the target's HP in half.

```json
{
  "effect": {
    "callbacks": {
      "on_move_damage": ["return func_call(max: expr($target.hp / 2) 1)"]
    }
  }
}
```

###### Move: Magnitude

Magnitude randomly selects a magnitude, which determines the move's base power. The magnitude selected is also made known to the user, after the move animates. We can use a custom move message for the latter.

In this case, we can actually just modify the move's base power directly. Later, base power modifiers will apply naturally.

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

##### Ability: Torrent

Torrent (and other similar abilities) boosts a Mon's attack stats for Water moves when the user has lost two-thirds of its HP. We can modify stats directly when they are calculated under Water-type moves.

```json
{
  "effect": {
    "callbacks": {
      "on_modify_atk": [
        "if $effect.is_defined and $effect.type == water and $target.hp <= expr($target.max_hp / 3):",
        ["return $atk * 3/2"]
      ],
      "on_modify_spa": [
        "if $effect.is_defined and $effect.type == water and $target.hp <= expr($target.max_hp / 3):",
        ["return $spa * 3/2"]
      ]
    }
  }
}
```

##### Ability: Sturdy

Sturdy prevents a Mon from being knocked out by a single move. When the Mon's HP is full, moves that _would_ knock out the Mon are altered to leave the target with 1 HP.

```json
{
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "if $move.ohko:",
        ["log_immune: $target from_effect", "return stop"]
      ],
      "on_damage": {
        "priority": -100,
        "program": [
          "if $target.hp == $target.max_hp and $damage > $target.hp and $effect.is_move:",
          ["log_activate: with_target", "return $target.hp - 1"]
        ]
      }
    }
  }
}
```

### Triggering Other Effects

Many effects trigger additional battle effects based on certain conditions being met. Additional effects can be triggered by calling functions that hook into the core battle engine. For example, `damage`, `heal`, `set_status`, `cure_status`, `add_volatile`, and `remove_volatile` are a few functions that operate on a target Mon. Many more battle functions exist for abilities, items, side conditions, weather, and more.

Effects may also wish to reveal themselves on the battle log, in order for clients to display some special animation or message. Several functions exist to add effect activation logs consistently, based on the context of the current effect callback.

- `log_ability` - A Mon's ability is announced. Only required if there is no other log when the ability activates.
- `log_activate` - An effect has activated, typically on some Mon but not necessarily. This is the most common function.
- `log_animate_move` - A move was used, but it should only be animated. Used for multihit moves.
- `log_block` - An effect blocked some other effect.
- `log_cant` - An effect is preventing a Mon from using a move.
- `log_custom_effect` - Logs an effect with a custom header. Should be used sparingly.
- `log_end` - An effect applying to a Mon has ended.
- `log_fail` - A move used by a Mon has failed.
- `log_fail_heal` - A Mon could not be healed.
- `log_fail_unboost` - Some stats of a Mon could not be lowered.
- `log_field_activate` - An effect (pseudo-weather) has activated on the field.
- `log_field_end` - An effect applying to the field has ended.
- `log_field_start` - An effect applying to the field has started.
- `log_immune` - A Mon is immune to the move being used.
- `log_ohko` - A Mon was OHKO'd by the move being used.
- `log_prepare_move` - A move is preparing to be used. It will be used at some later time.
- `log_side_end` - An effect applying to a side has ended.
- `log_side_start` - An effect applying to a side has started.
- `log_single_move` - An effect has activated and is active until the target Mon uses another move.
- `log_single_turn` - An effect has activated and is active for the rest of the turn.
- `log_start` - An effect applying to a Mon has started.
- `log_status` - A non-volatile status condition has been applied to a Mon.
- `log_weather` - A weather effect has been applied to the field.

#### Modifying Forwarded Effects

Most functions that trigger additional battle effects or add effect activation logs use the **current evaluation context** for the forwarded effect context. For example, when calling `cure_status`, the source effect of that call becomes the _current_ effect callback being used. To be more concrete, below is the effect for the move Wake-Up Slap, which wakes up the target if it is asleep when hit:

```json
{
  "effect": {
    "callbacks": {
      "on_move_base_power": [
        "if $target.is_asleep:",
        ["return $move.base_power * 2"]
      ],
      "on_hit": ["if $target.status == slp:", ["cure_status: $target"]]
    }
  }
}
```

The call to `cure_status: $target` will trigger additional events (like `CureStatus`). When these execute, the source effect variable (e.g., `$effect`) will be set to the Wake-Up Slap active move. This allows effect callbacks to see what the source effect is and even prevent the target from waking up. For example, we could make a "Deep Sleep" ability that prevents opponent Mons from waking up from Wake-Up Slap. Its effect could look as follows:

```json
{
  "effect": {
    "callbacks": {
      "on_foe_cure_status": ["if $effect.id == wakeupslap:", ["return false"]]
    }
  }
}
```

However, there may be cases where we _do not_ want to use _this_ effect as the source effect of our function calls. Maybe we want our effect to be transparent, so it is as if the current source effect triggered it. A great example is when a move thaws out a frozen target. Curing logic is not implemented on a per-move basis; some moves explicitly thaw a target, some moves thaw the user, and generically, any Fire-type move thaws out its target after damage is dealt.

These conditions are implemented generically in the "Freeze" condition. However, it is not the "Freeze" condition thaws the Mon; the move itself (the source effect) is the one triggering the effect. Thus, we do the following:

```json
{
  "condition": {
    "callbacks": {
      "on_use_move": [
        "if func_call(move_has_flag: $move thawing):",
        ["cure_status: $user use_source_effect"]
      ],
      "on_after_move_secondary_effects": [
        "if $move.thaws_target:",
        ["cure_status: $target use_source_effect"]
      ],
      "on_damaging_hit": [
        "if $move.type == fire and $move.category != status:",
        ["cure_status: $target use_source_effect"]
      ]
    }
  }
}
```

The `use_source_effect` tag is a special string parameter interpreted by effect functions that uses the current source effect for the forwarded effect context. _This_ effect becomes transparent and is not forwarded. In the example above, any `CureStatus` callback that runs will see the thawing move as its source effect, _not_ the "Freeze" condition.

Additional flags (special string parameters) exist that work across a variety of effect and logging functions.

Generic tags:

- `no_source` - The source Mon is ignored.
- `no_source_effect` - The source effect is ignored.
- `use_effect_as_source_effect` - This effect is used as the source effect.
- `use_effect_state_source` - `$effect_state.source` is used as the target Mon.
- `use_effect_state_source_as_source` - `$effect_state.source` is used as the source Mon.
- `use_effect_state_source_effect` - `$effect_state.source_effect` is used as this effect.
- `use_effect_state_target` - `$effect_state.target` is used as the target Mon.
- `use_effect_state_target_as_source` - `$effect_state.target` is used as the source Mon.
- `use_source` - The source Mon is used as the target Mon.
- `use_source_effect` - The source effect is used as this effect.
- `use_target_as_source` - The target Mon is used as the source Mon.

Logging tags:

- `no_effect` - This effect is not logged.
- `with_source` - The source Mon is logged.
- `with_source_effect` - The source effect is logged.
- `with_target` - The target Mon is logged.

#### Examples

##### Move: Splash

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

##### Move: Pain Split

Pain Split does not deal damage in the traditional sense; instead, it takes the average HP between the user and the target and sets both Mon's HP directly.

```json
{
  "effect": {
    "callbacks": {
      "on_hit": [
        "$target_hp = $target.hp",
        "$average_hp = func_call(max: 1 expr(($target.hp + $source.hp) / 2))",
        "$target_diff = $target_hp - $average_hp",
        "set_hp: $target $average_hp",
        "set_hp: $source $average_hp"
      ]
    }
  }
}
```

##### Move: Tri-Attack

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

##### Ability: Speed Boost

Speed Boost raises the Mon's speed stat at the end of each turn.

```json
{
  "effect": {
    "callbacks": {
      "on_residual": [
        "if $target.active_turns > 0:",
        ["boost: $target 'spe:1'"]
      ]
    }
  }
}
```

##### Ability: Volt Absorb

Volt Absorb heals the Mon any time it is hit with an Electric-type move.

In the callback below, we heal the Mon with `use_target_as_source`. This tag causes the target Mon (the one with the ability, being hit) to be used as the source of the healing. Without this tag, the source of the effect would be used, which is the move user, which would result in the `heal` log to say the healing came from the move user. However, this is not entirely true; the healing came from the target's ability. Thus, `use_target_as_source` makes both logical sense, and prevents the move user from being output on the `heal` log.

```json
{
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "if $target != $source and $move.type == electric:",
        [
          "if func_call(heal: $target expr($target.base_max_hp / 4) use_target_as_source) == 0:",
          ["log_immune: $target from_effect"],
          "return stop"
        ]
      ]
    }
  }
}
```

##### Ability: Intimidate

When the Intimidate ability activates, all adjacent foes have their attack lowered by one stage.

We include `with_target` in the `log_activate` call in order for the effect activation log to include the target of this callback, which is the Mon with the ability. Additionally, `use_target_as_source` in the `boost` call ensures that effects triggered by the stat change has the correct source Mon.

```json
{
  "effect": {
    "callbacks": {
      "on_start": [
        "$activated = false",
        "foreach $mon in func_call(adjacent_foes: $target):",
        [
          "if !$activated:",
          ["log_activate: with_target", "$activated = true"],
          "boost: $mon 'atk:-1' use_target_as_source"
        ]
      ]
    }
  }
}
```

##### Ability: Static

Static has a change to paralyze a move user after a Mon is hit by a move that makes contact.

`use_target_as_source` serves the same purpose here: effect callbacks should include the correct source Mon (the target of this effect, which is the Mon with the ability). It also ensures the log for the status being applied to the move user contains the Mon with the ability.

```json
{
  "effect": {
    "callbacks": {
      "on_damaging_hit": [
        "if func_call(move_makes_contact: $move) and func_call(chance: 3 10):",
        ["set_status: $source par use_target_as_source"]
      ]
    }
  }
}
```

##### Item: Life Orb

Life Orb boosts the base power of all moves used by the holder, but it damages the holder after each move.

```json
{
  "effect": {
    "callbacks": {
      "on_modify_damage": ["return $damage * 13/10"],
      "on_after_move": [
        "if $target.is_defined and $user != $target and $move.category != status and !$user.force_switch:",
        ["damage: $user expr($user.base_max_hp / 10) use_target_as_source"]
      ]
    }
  }
}
```

##### Item: Cheri Berry

Cheri Berry, when eaten, heals the Mon of paralysis.

```json
{
  "effect": {
    "callbacks": {
      "on_player_try_use_item": ["if $target.status != par:", ["return false"]],
      "on_player_use": ["eat_given_item: $mon $this.id"],
      "on_update": ["if $mon.status == par:", ["eat_item: $mon"]],
      "on_eat": ["if $mon.status == par:", ["cure_status: $mon"]]
    }
  }
}
```

There are several callbacks at play here:

- `PlayerTryUseItem` triggers when a player is _trying_ to use a Cheri Berry on a Mon directly. A player can only use the item on a Mon that is paralyzed.
- `PlayerUse` triggers when a player has used a Cheri Berry on a Mon directly. The Mon eats the item.
- `Update` triggers at several points during a turn, mostly after each move. If a Mon is paralyzed, it eats its item (this berry).
- `Eat` triggers when a Mon eats a Cheri Berry. If it is paralyzed, it will be cured.

It may seem like we check the Mon for paralysis several times, but each check is important.

- If we did not check for paralysis in the `PlayerTryUseItem` callback, then players could use a Cheri Berry on Mons even if it would be completely wasted.
- If we did not check for paralysis in the `Update` callback, then Mons would eat its berry immediately and waste it.
- If we did not check for paralysis in the `Eat` callback, then Mons that eat the berry through other means (e.g., Pluck, Fling) would be cured of _any_ status.

### Prolonged Effects (Conditions)

A **condition** is an effect that attaches to a part of the battle to provide prolonged effects. Conditions can last until a Mon switches out, until the effect is forcefully removed, or until a certain number of turns have passed.

There are several types of conditions:

- Non-volatile status conditions attach to a Mon, one at a time.
- Volatile status conditions attach to a Mon until they switch out.
- Side conditions attach to a side of the battle.
- Slot conditions attach to a position on a side of the battle.
- Weathers attach to the field, one at a time.
- Terrains attach to the field, one at a time.
- Pseudo-weathers (field conditions) attach to the field.

Conditions can define a **duration**, which is a number of turns after which the condition will automatically be removed from its target. Conditions without a target exist indefinitely until removed directly (such as by some effect callback) or indirectly (such as by a Mon switching out, for example, which removes volatile status conditions).

Conditions can be defined on most effects _in addition to_ their core effect. For example, a move can have both an `effect` and a `condition`, both of which have a disjoint set of effect callbacks that trigger. The move effect would trigger when the move is used; the move condition would trigger when it is attached to a Mon/side/slot/field. Typically, moves attach their own conditions, but they can also attach different conditions (e.g., a move can apply a non-volatile status condition, like "Paralysis" or "Sleep").

#### Examples

##### Move: Roost

Roost heals 50% of the user's HP. For the rest of the turn, the Mon loses its Flying type.

The move applies the "roost" volatile status, which just refers to the condition defined on the same move. This condition has a duration of "1", which means it lasts one turn. In other words, it only lasts until the end of _this_ turn, since the counter will reach 0 and force the condition to be removed. The condition then implements a `Types` callback that removes the Flying type.

```json
{
  "hit_effect": {
    "heal_percent": "50%",
    "volatile_status": "roost"
  },
  "condition": {
    "duration": 1,
    "callbacks": {
      "on_start": ["log_single_turn: with_target"],
      "on_types": ["return func_call(remove: $types flying)"]
    }
  }
}
```

##### Move: Aqua Ring

Aqua Ring adds a volatile status to the target (a.k.a., the user) that heals HP at the end of each turn. This condition has _no_ duration, so it exists until it is removed. Since nothing removes Aqua Ring directly, the condition effectively lasts until the Mon switches out (when all volatile statuses are removed).

```json
{
  "hit_effect": {
    "volatile_status": "aquaring"
  },
  "condition": {
    "callbacks": {
      "on_start": ["log_start"],
      "on_residual": ["heal: $target expr($target.base_max_hp / 16)"]
    }
  }
}
```

##### Ability: Flash Fire

When a Mon has the Flash Fire ability, if it is hit by a Fire-type move, it is immune and receives a boost to its attack stats. The boost is implemented as a volatile status, not a traditional stat boost.

`no_copy` means that the condition cannot be passed to other Mons by effects like "Baton Pass."

```json
{
  "effect": {
    "callbacks": {
      "on_start": [
        "if $effect_state.activated:",
        ["add_volatile: $target $this.id"]
      ],
      "on_try_hit": [
        "if $target == $source or $move.type != fire:",
        ["return"],
        "$move.accuracy = exempt",
        "if !func_call(add_volatile: $target $this.id):",
        ["log_immune: $target from_effect"],
        "$effect_state.activated = true",
        "return stop"
      ],
      "on_end": ["remove_volatile: $target $this.id"]
    }
  },
  "condition": {
    "no_copy": true,
    "callbacks": {
      "on_start": [
        "$ability_effect_state = func_call(ability_effect_state: $target)",
        "if $ability_effect_state.is_defined and $ability_effect_state.activated:",
        ["log_start: silent", "return"],
        "log_start"
      ],
      "on_end": ["log_end: silent"],
      "on_modify_atk": [
        "if $effect.is_defined and $effect.type == fire:",
        ["return $atk * 3/2"]
      ],
      "on_modify_spa": [
        "if $effect.is_defined and $effect.type == fire:",
        ["return $spa * 3/2"]
      ]
    }
  }
}
```

We keep track of `$effect_state.activated` for ability suppression. When an ability is suppressed, the `End` event runs. When an ability is unsuppressed, the `Start` event runs. The volatile status is only given to the Mon when the ability is active and unsuppressed.

##### Status: Paralysis

Paralysis is an independent condition; it is not attached to any single move or ability. It exists on its own and can be applied by any other effect. Its effects are quite straightforward.

```json
{
  "condition": {
    "callbacks": {
      "on_start": ["log_status: $this.name"],
      "on_before_move": {
        "priority": 1,
        "program": ["if func_call(chance: 1 4):", ["log_cant", "return false"]]
      },
      "on_modify_spe": [
        "if !func_call(has_ability: $target quickfeet):",
        ["return $spe / 2"]
      ],
      "on_modify_catch_rate": ["return $catch_rate * 3/2"]
    }
  }
}
```

##### Volatile Status: Confusion

Similar to Paralysis, Confusion is also a generic condition applied as a volatile status. It has a concept of duration (ranging from two to six turns). However, it is not implemented with the traditional `duration` field. A Mon "snaps out of confusion" right before it uses a move. Thus, the effect duration is decremented and checked manually every time a Mon is using a move.

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
          "damage: $user $damage no_source",
          "return false"
        ]
      }
    }
  }
}
```

##### Move: Perish Song

Perish Song is not _exactly_ a field condition. The move hits the entire field, but it then hits each Mon individually and applies a volatile status to them that causes them to faint after three turns.

The special `prepare_direct_move` function runs all the pre-move logic that is normally run for a move such as accuracy checks and move-modifying effects. While Perish Song does bypass accuracy, some Mons may still be invulnerable to the move, so this filter is important.

The condition itself is simple; after four turns, the condition ends and the Mon faints.

```json
{
  "effect": {
    "callbacks": {
      "on_hit_field": [
        "$success = false",
        "$activate = false",
        "$targets = func_call(all_active_mons)",
        "if $targets.is_empty:",
        ["return false"],
        "foreach $target in func_call(prepare_direct_move: $targets):",
        [
          "$success = true",
          "# Activate if at least one Mon did not already have this status.",
          "$hit_target = func_call(add_volatile: $target $this.id)",
          "$activate = $activate or $hit_target"
        ],
        "if !$success or !$activate:",
        ["return false"],
        "if $activate:",
        ["log_field_activate"]
      ]
    }
  },
  "condition": {
    "duration": 4,
    "callbacks": {
      "on_residual": ["log_start: str('perish:{}', $effect_state.duration)"],
      "on_end": ["log_start: 'perish:0'", "faint: $target"]
    }
  }
}
```

##### Move: Light Screen

Light Screen halves the damage taken by Special moves on the user's side of the battle. This move condition applies to an entire side, so the `SourceModifyDamage` callback will run for every Mon on the side where Light Screen is active.

```json
{
  "hit_effect": { "side_condition": "lightscreen" },
  "condition": {
    "duration": 5,
    "callbacks": {
      "on_source_modify_damage": [
        "if $target == $user or $move.category != special:",
        ["return"],
        "if func_call(move_crit_target: $move $target) or $move.effect_state.infiltrates:",
        ["return"],
        "if $format.mons_per_side > 1:",
        ["return $damage * 2 / 3"],
        "return $damage / 2"
      ],
      "on_side_start": ["log_side_start"],
      "on_side_end": ["log_side_end"]
    }
  }
}
```

##### Move: Healing Wish

Healing Wish adds a slot condition, which is a condition on a single position in battle. When a Mon needing healing switches in to the slot where Healing Wish was used, the wish is consumed and the Mon is healed.

Healing Wish removes itself after it is used. Otherwise, it exists until consumed.

```json
{
  "self_destruct": "ifhit",
  "hit_effect": {
    "slot_condition": "healingwish"
  },
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "if !func_call(can_switch: $target.player):",
        ["return false"]
      ]
    }
  },
  "condition": {
    "callbacks": {
      "on_switch_in": [
        "if $mon.fainted or ($mon.hp >= $mon.max_hp and !$mon.status):",
        ["return"],
        "log_activate: with_target",
        "heal: $mon $mon.max_hp",
        "cure_status: $mon",
        "remove_slot_condition: $mon.side $mon.position $this.id"
      ]
    }
  }
}
```

##### Move: Gravity

Gravity is a true field condition, also known as a pseudo-weather. It disables certain moves, improves accuracy, grounds Mons, and even cancels ongoing moves like Fly or Bounce.

Note the concept of being grounded, implemented as a state event callback below, is explored as a later topic.

```json
{
  "hit_effect": {
    "pseudo_weather": "gravity"
  },
  "condition": {
    "duration": 5,
    "callbacks": {
      "on_field_start": ["log_field_start"],
      "on_modify_accuracy": ["return $acc * 5/3"],
      "on_disable_move": [
        "foreach $move_slot in $mon.move_slots:",
        [
          "if func_call(move_has_flag: $move_slot.id gravity):",
          ["disable_move: $mon $move_slot.id"]
        ]
      ],
      "is_grounded": {
        "priority": 1,
        "program": ["return true"]
      },
      "on_before_move": [
        "if func_call(move_has_flag: $move.id gravity):",
        ["log_cant", "return false"]
      ],
      "on_use_move": [
        "if func_call(move_has_flag: $move.id gravity):",
        ["log_cant", "return false"]
      ],
      "on_field_end": ["log_field_end"]
    }
  }
}
```

## Advanced Topics

### Effect Handles vs. IDs

There are two primary ways effects are identified in the battle engine and fxlang programs: handles and IDs.

- An **effect handle** references some specific type of effect. It can refer to the static definition of an effect by _type_ and _ID_; or it can refer to a dynamic instantiation of an effect (e.g., an active move is an instantiation of a static move that allows the move data to be modified per use).
- **Effect IDs** are a normalized string, that may refer to any type of effect. The type of the effect is contextual.

In the battle engine, any stored references to effects are typically held by ID. For example, `$mon.ability` is a string effect ID. The ability effect itself can be looked up with `func_call(get_ability: $mon.ability)`, which returns an effect handle that allows the actual ability data to be referenced. On the other hand, variables like `$this` and `$effect` are always effect handles, and their ID can be referenced with `$effect.id`.

Most functions take in an ID for simplicity, since it is typically always available. Effect handles are needed only when effect data must be referenced, or when using an effect as a parameter for effect forwarding.

### Local Data

An effect can define a `local_data` object, which is data that can be referenced and constructed in effect callbacks to simplify dynamic effects. Currently, `local_data` consists of a map of local moves that can be instantiated and used from within the effect callback, using the `new_active_move_from_local_data` function.

#### Examples

##### Move: Bide

Bide stores up damage applied to the user for several turns. On the third turn, the user attacks the Mon that last damaged it with twice the stored damage. When Bide is first used, it applies the "bide" volatile status to the user, which implements all damage-storing event callbacks.

To unleash the damage, Bide uses a custom version of itself that applies the stored damage to the target. The custom version of Bide is stored in the condition's `local_data`.

The benefit here is that the modified move can be written statically in the condition code, rather than dynamically inside the event callback (pretty much every field would need to be overwritten). Furthermore, this customized version of Bide can actually have its _own_ event callbacks. In this case, the `TryUseMove` callback fails the move if no damage would be applied. And by setting `no_random_target`, the move also fails if the Bide volatile status condition did not have any target for the move. Thus, the two ways of failing the move are covered directly in the core battle engine rather than in the dynamic event callback code.

```json
{
  "hit_effect": { "volatile_status": "bide" },
  "condition": {
    "duration": 3,
    "callbacks": {
      "on_start": ["$effect_state.total_damage = 0", "log_start"],
      "on_restart": ["return true"],
      "on_lock_move": ["return $this.id"],
      "on_damaging_hit": [
        "if $source.is_defined and $source != $target:",
        ["$effect_state.last_damage_source = $source"],
        "$effect_state.total_damage = $effect_state.total_damage + $damage"
      ],
      "on_before_move": [
        "# This callback runs when the user is storing energy.",
        "if $effect_state.duration > 1:",
        ["log_activate: with_target", "return"],
        "# Bide is ending this turn, so this use of the move unleashes the energy.",
        "log_end",
        "$target = $effect_state.last_damage_source",
        "# Create a new active move that deals the damage to the target, and use it directly.",
        "$move = func_call(new_active_move_from_local_data: $this $this.id)",
        "$move.damage = expr($effect_state.total_damage * 2)",
        "# Remove this volatile status before using the new move, or else this callback gets triggered endlessly.",
        "remove_volatile: $user $this.id",
        "use_active_move: $user $move $target no_source_effect",
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

### State Events

A state event is a special type of battle event that represents the state of some entity, rather than some effect or calculation. A state event callback is intended to be simple, only returning a boolean result. A state event terminates as soon as some effect returns a result. In other words, as soon as we find an effect that returns "true" or "false" for a state event, that decision is used as the final result.

State events allow effects to modify complex states directly with fxlang, like any other effect callback. The order in which effects modify a state can be determined by the same ordering rules as any other event.

#### Examples

##### Grounded State

When a Mon is grounded, it can be affected by Ground-type moves. A Mon is grounded by default, but it can become ungrounded by several means, such as having the Flying type, having the ability Levitate, or being under the effect of a move like Magnet Rise.

The `IsGrounded` event is used to check if a Mon is grounded. Remember, the result of this state event is determined by the _first_ callback that returns a value. A callback can return no value (`undefined`) if it wishes to defer to some other callback for the decision.

The default grounded state (`true`) is hard-coded in the battle engine itself. Several effects can integrate with the `IsGrounded` event to overwrite this value.

Flying types are ungrounded by default:

```json
{
  "condition": {
    "callbacks": {
      "is_grounded": {
        "priority": -100,
        "program": ["return false"]
      }
    }
  }
}
```

The Levitate ability also causes a Mon to be ungrounded:

```json
{
  "effect": {
    "callbacks": {
      "is_grounded": ["return false"]
    }
  }
}
```

However, the condition induced by the move Ingrain overwrites both of these effects, grounding a Mon:

```json
{
  "hit_effect": {
    "volatile_status": "ingrain"
  },
  "condition": {
    "callbacks": {
      "is_grounded": {
        "order": 1,
        "program": ["return true"]
      },
      "on_start": ["log_start"],
      "on_residual": ["heal: $target expr($target.base_max_hp / 16)"],
      "on_trap_mon": ["return true"],
      "on_drag_out": ["log_activate: with_target", "return false"]
    }
  }
}
```

Thus, a Mon's grounded state need not be hardcoded into the battle engine or some list of complex effects and interactions. Like any other battle event, active effect callbacks are collected, ordered, and executed to determine a Mon's state.

##### Weather Suppression

Weather is a condition that acts on the entire field. However, the effects of weather can be **suppressed** on individual Mons. When suppressed, all effects of weather (positive and negative) are ignored and not executed. A Mon's weather suppression state, like the grounded state, is a state event that effects can attach callbacks to.

Let's explore how weather suppression can work in a battle. First, let's define the Rain weather, which is fairly straightforward to implement:

```json
{
  "condition": {
    "callbacks": {
      "is_raining": ["return true"],
      "on_duration": [
        "if !$source:",
        ["return"],
        "if func_call(has_item: $source damprock):",
        ["return 8"],
        "return 5"
      ],
      "on_source_weather_modify_damage": [
        "# Run against the target of the damage calculation, since weather can be suppressed for the target.",
        "if $move.type == water:",
        ["return $damage * 3/2"],
        "if $move.type == fire:",
        ["return $damage * 1/2"]
      ],
      "on_field_start": ["log_weather: $this.name with_source_effect"],
      "on_field_residual": {
        "order": 1,
        "priority": 1,
        "program": ["log_weather: $this.name residual"]
      },
      "on_residual": {
        "order": 1,
        "program": ["run_event: Weather"]
      },
      "on_field_end": ["log_weather"]
    }
  }
}
```

Some notes about the above code:

1. The `IsRaining` event is a state event that only runs for the weather on the field. Other effects can check for this property (which will trigger this state event) without needing to explicitly check for all weathers that include rain (for instance, Primordial Sea causes a different type of rain but many of the same side effects apply).
1. The `Duration` callback returns no value if the weather did not originate from any source Mon. This allows the effect to be used as the "default weather" of the field (imagine battles that start when it's rainy in the overworld).
1. Using "source" in the damage modification event means it runs when a Mon is being targeted. This is because damage modifications due to rain only apply if the _target_ is under rain.

###### Damage Modification with Suppression

We will examine how Rain's damage modification works with a complex example. Consider the following scenario:

1. It is raining.
1. Blastoise uses Water Gun against Charizard.
1. Charizard is holding a Utility Umbrella.
1. Charizard is under the effect of Embargo.

Normally, Blastoise's Water Gun should get a 50% damage boost against Charizard because of the rain. However, since Charizard is holding a Utility Umbrella, the effects of rain are suppressed for Charizard, so the damage effect should not apply. However, Embargo negates the effects of the target's held item, so the rain modification _should_ actually apply!

As you can see, there are two layers of suppression happening here:

1. Utility Umbrella suppresses rain.
1. Embargo suppresses Utility Umbrella.

This plays itself out in battle code completely naturally.

First, the rain weather declares it is raining.

```json
{
  "is_raining": ["return true"]
}
```

Second, the Utility Umbrella item declares that it suppresses weather if it is raining:

```json
{
  "suppress_mon_weather": [
    "if $field.weather.is_defined and ($field.weather.is_raining or $field.weather.is_sunny):",
    ["return true"]
  ]
}
```

Third, Embargo declares that it suppresses the target's item:

```json
{
  "suppress_mon_item": ["return true"]
}
```

These suppression events are checked in the fxlang evaluation code directly. Thus, callbacks for the weather effect will not run if the Mon's weather is suppressed, and callbacks for the Mon's item effect will not run if its item is suppressed.

The end result is that the rain weather fxlang code **does not** need to care about any of this! It trusts that the core battle engine only executes its callback when it truly applies.

Then, moves that have side effects based on the presence of rain can easily integrate with this complex suppression. For example, consider the accuracy of the move Thunder:

```json
{
  "effect": {
    "callbacks": {
      "on_use_move": [
        "$weather = $selected_target.effective_weather",
        "if !$weather:",
        ["return"],
        "if $weather.is_raining:",
        ["$move.accuracy = exempt"],
        "else if $weather.is_sunny:",
        ["$move.accuracy = 50"]
      ]
    }
  }
}
```

The term "effective" is used to look up some property where that property may be suppressed. Abilities, items, weathers, and terrains can all be suppressed by various effects. Effects can choose to honor suppression or not, depending on what they do.

### Reusing Common Effects

#### With Independent Conditions

It is often extremely useful to reuse common conditions and callbacks for a variety of effects. We have already seen how we enable this sharing for non-volatile status conditions: their condition is defined independent of any move or ability, and then these types of effects can inflict the common status on a Mon. As an example, the "Paralysis" condition is well-defined and inflicted across a wide variety of effects.

We can extend this idea of reusability for _any_ type of effect. We can define independent volatile status conditions and attach them wherever we see fit.

For example, many moves force the user to recharge on their next turn: Hyper Beam, Giga Impact, Frenzy Plant, etc. Instead of implementing recharge directly into the core battle engine, we can make "Must Recharge" a volatile status condition that the user receives after using a recharge move. We define this condition below:

```json
{
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": ["log_activate: with_target"],
      "on_before_move": {
        "priority": 11,
        "program": [
          "log_cant",
          "remove_volatile: $user $this.id",
          "return false"
        ]
      },
      "on_lock_move": ["return recharge"]
    }
  }
}
```

Then, we only need to add the volatile to the user for any recharge move:

```json
{
  "user_effect": {
    "volatile_status": "mustrecharge"
  }
}
```

[`conditions.json`](./battle-data/data/conditions.json) contains definitions for all conditions that are intended to be generic and reusable. Several of them will be explained in following sections.

#### With Delegate Effects

Another way we can reuse common effects is with delegate effects. The `delegates` field of an effect can list one or more effect IDs. When an effect is parsed, effect callbacks of delegate effects are simply _imported_ into the target effect. In other words, delegate effect callbacks are copy-pasted into the target effect. This makes reuse much more explicit.

Delegate effects are primarily useful for reusing callbacks specific to moves. Events like `TryUseMove` or `MoveBasePower` never run on a condition, but defining a delegate effect allows another move (or condition)'s code to be copied directly.

For example, the move "Ice Ball" is an exact duplicate of the move "Rollout". We could define a shared condition, but the Rollout condition calculates the move's base power directly. Rather than defining the complex move and condition twice, we can have Ice Ball simply delegate to Rollout:

```json
{
  "effect": {
    "delegates": ["move:rollout"]
  },
  "condition": {
    "delegates": ["movecondition:rollout"]
  }
}
```

#### Examples

##### Locked Move

Some moves lock the user into the same move for several turns and inflict the user with confusion due to fatigue afterwards. Thrash, Outrage, and Petal Dance all have this same effect. This condition of being locked into a move is implemented as a common volatile status condition:

```json
{
  "condition": {
    "callbacks": {
      "on_duration": ["return func_call(random: 2 4)"],
      "on_start": ["$effect_state.move = $source_effect.id"],
      "on_after_move": [
        "if $user.move_this_turn_failed and $effect_state.duration > 1:",
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

Then a move like Thrash is simple:

```json
{
  "user_effect": {
    "volatile_status": "lockedmove"
  }
}
```

Here are some implementation notes:

- We save the locked move on the effect state of the volatile for locking the Mon into the move.
- We use `remove_volatile` to end the effect early after the Mon moves. This causes confusion to be inflicted immediately, rather than waiting for the end of the turn where the volatile would be removed naturally.

##### Charge Moves

Several moves are considered "charge moves" that run over two turns. On the first turn, the Mon charges up. On the second turn, the Mon unleashes a powerful blow and finishes the move.

To implement charge moves generically, we use both reuse mechanisms described above. Charge moves delegate to a base condition for the logic that ensures the move executes over two turns, and a common volatile status condition ensures we are properly locked into that move for two turns.

###### Charge Move Base

This base defines a common `TryUseMove` callback that is imported by any charge move.

The callback knows we are in the second turn of the move if we _already_ have the move condition volatile (a.k.a., `remove_volatile` returns `true`). If this returns false, it means we do not have the move condition, which means we have not yet charged up the move, so we are on the first turn.

The `ChargeMove` event is a special event that a move can implement to do something when charging or even skip charging altogether.

```json
{
  "condition": {
    "callbacks": {
      "on_try_use_move": [
        "if func_call(remove_volatile: $user $this.id):",
        ["return"],
        "log_prepare_move",
        "$charge_move = func_call(run_event_on_move: ChargeMove)",
        "if $charge_move.is_defined and !$charge_move:",
        ["return"],
        "if !func_call(run_event: ChargeMove):",
        ["return"],
        "add_volatile: $user twoturnmove link",
        "return stop"
      ]
    }
  }
}
```

Skull Bash uses this move base quite elegantly, using the `ChargeMove` event to boost the user's defense:

```json
{
  "effect": {
    "delegates": ["condition:chargemovebase"],
    "callbacks": {
      "on_charge_move": ["boost: $user 'def:1'"]
    }
  }
}
```

Solar Beam is a bit more complex; in sunny weather, the move is executed immediately. In other weathers, the move is weakened.

```json
{
  "effect": {
    "delegates": ["condition:chargemovebase"],
    "callbacks": {
      "on_charge_move": [
        "$weather = $user.effective_weather",
        "if $weather.is_defined and $weather.is_sunny:",
        [
          "do_not_animate_last_move",
          "log_animate_move: $user $this.name $target",
          "return false"
        ]
      ],
      "on_move_base_power": [
        "$weak_weathers = [rainweather, heavyrainweather, sandstormweather, hailweather, snowweather]",
        "if $weak_weathers has $source.effective_weather:",
        ["return $move.base_power * 1/2"]
      ]
    }
  }
}
```

##### Two Turn Move Volatile

The move base above applies the "Two Turn Move" volatile status condition. This condition is also defined generically:

```json
{
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": [
        "# Note that the $target here is the user of the move (target of this condition).",
        "$effect_state.move = $source_effect.id",
        "add_volatile: $target $effect_state.move link",
        "# If this move is called by another move, we may need to modify the target that the user will be locked into.",
        "# For example, Metronome targets the user, but Razor Wind targets adjacent foes.",
        "#",
        "# ... skipped logic for retargeting when called by other moves like Metronome."
        "#",
        "do_not_animate_last_move",
        "# Still run events associated with the user preparing to hit the target, since they are locked into this move.",
        "run_event: PrepareHit"
      ],
      "on_set_last_move": ["if $effect_state.duration > 1:", ["return false"]],
      "on_deduct_pp": {
        "priority": -999,
        "program": [
          "# Run last, to ensure no PP is deducted while charging.",
          "if $effect_state.duration > 1:",
          ["return 0"]
        ]
      },
      "on_lock_move": ["return $effect_state.move"],
      "on_move_aborted": ["remove_volatile: $user $effect_state.move"]
    }
  }
}
```

##### Move Condition Volatile

The "Two Turn Move" condition immediately applies the volatile status condition for the move being used. Think of the move Fly: the user goes up in the air and is semi-invulnerable to most moves. This allows us to define the Fly condition quite elegantly:

```json
{
  "effect": {
    "delegates": ["condition:chargemovebase"]
  },
  "condition": {
    "duration": 2,
    "callbacks": {
      "is_semi_invulnerable": ["return true"],
      "on_invulnerability": [
        "if [gust, twister, skyuppercut, thunder, hurricane, smackdown, thousandarrows] has $move.id:",
        ["return"],
        "return false"
      ],
      "on_source_modify_damage": [
        "if [gust, twister] has $move.id:",
        ["return $damage * 2"]
      ]
    }
  }
}
```

##### Putting It All Together

With the conditions above defined, we have all the pieces for implementing charge moves:

1. Charge moves delegate to a "Charge Move Base" condition that has a special `TryUseMove` callback.
1. On the first turn, the user receives a "Two Turn Move" volatile status condition. This condition means the Mon is locked into the charge move.
1. The "Two Turn Move" condition adds the move condition to the user, representing that the Mon is in the charging state for that move.
1. After these volatile status conditions are added on the first turn, the move is interrupted and does not execute.
1. On the second turn, the conditions above are removed, and the move fully executes.

### Explaining Complex Effects

This section goes over complex effects solely for the purpose of explaining how they are implemented with fxlang.

#### Examples

##### Move: Substitute

Substitute is a bit of an exception, since it does things unlike any other move in the battle engine. Substitute takes a quarter of the user's HP and applies it to a substitute. That substitute has the same amount of HP and will absorb all hit effects (damage, stat boosts/drops, statuses, volatiles). Once the substitute runs out of HP, it will disappear, and the Mon will be hittable again.

Since Substitute is so niche, it actually gets its own event, `TryPrimaryHit`, that allows a move to completely override how a move is applied to a Mon. If a callback for this event returns 0, the core battle engine assumes a substitute was hit, and the target becomes exempt from any other effect of the move.

Here is the code in all of its glory:

```json
{
  "hit_effect": { "volatile_status": "substitute" },
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "if func_call(has_volatile: $source substitute) or $source.hp <= $source.max_hp / 4 or $source.max_hp == 1:",
        ["log_fail: $source", "return stop"]
      ],
      "on_hit": ["direct_damage: $target expr($target.max_hp / 4)"]
    }
  },
  "condition": {
    "callbacks": {
      "is_behind_substitute": ["return true"],
      "on_start": [
        "log_start",
        "$effect_state.hp = func_call(floor: expr($target.max_hp / 4))",
        "if func_call(has_volatile: $target partiallytrapped):",
        ["remove_volatile: $target partiallytrapped"]
      ],
      "on_try_primary_hit": [
        "# Some moves can hit through substitute.",
        "if $target == $source or func_call(move_has_flag: $move bypasssubstitute) or $move.effect_state.infiltrates:",
        ["return"],
        "save_move_hit_data_flag_against_target: $move $target hitsubstitute",
        "# Calculate and apply damage.",
        "$damage = func_call(calculate_damage: $target)",
        "if $damage.is_boolean and !$damage:",
        ["log_fail: $source", "do_not_animate_last_move", "return false"],
        "if $damage > $effect_state.hp:",
        ["$damage = $effect_state.hp"],
        "$effect_state.hp = $effect_state.hp - $damage",
        "$move.total_damage = $move.total_damage + $damage",
        "# Break the substitute when HP falls to 0.",
        "if $effect_state.hp == 0:",
        [
          "if $move.ohko:",
          ["log_ohko: $target"],
          "remove_volatile: $target substitute"
        ],
        "else:",
        ["log_activate: with_target damage"],
        "# Some move effects still apply.",
        "apply_recoil_damage: $damage",
        "apply_drain: $source $target $damage",
        "run_event_on_move: AfterSubstituteDamage",
        "run_event: AfterSubstituteDamage",
        "return 0"
      ],
      "on_try_boost": [
        "if $target == $source:",
        ["return"],
        "log_fail_unboost: $target from_effect",
        "return func_call(boost_table)"
      ],
      "on_end": ["log_end"]
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

##### Stalling Moves

Stalling moves share a common behavior: their chance of success drops steeply between consecutive uses. Moves like Protect, Detect, and Endure all share the same accuracy check. To be specific, using Endure immediately after Protect has the same chance of failing if Protect is used twice in a row.

To represent this shared state, we simply add a volatile status condition to the Mon after it uses a stalling move.

For instance, here is the code for Protect:

```json
{
  "hit_effect": {
    "volatile_status": "protect"
  },
  "effect": {
    "callbacks": {
      "on_prepare_hit": [
        "return func_call(any_mon_will_move_this_turn) and func_call(run_event_for_mon: StallMove)"
      ],
      "on_hit": ["add_volatile: $target stall"]
    }
  },
  "condition": {
    "duration": 1,
    "callbacks": {
      "on_start": ["log_single_turn: with_target"],
      "on_try_hit": {
        "priority": 3,
        "program": [
          "if !func_call(move_has_flag: $move protect):",
          ["return"],
          "log_activate",
          "return stop"
        ]
      }
    }
  }
}
```

Then here is the code for the stall condition:

```json
{
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": ["$effect_state.counter = 3"],
      "on_restart": [
        "if $effect_state.counter < 729:",
        ["$effect_state.counter = $effect_state.counter * 3"],
        "$effect_state.duration = 2"
      ],
      "on_stall_move": [
        "$success = func_call(chance: $effect_state.counter)",
        "if !$success:",
        ["remove_volatile: $mon $this.id"],
        "return $success"
      ]
    }
  }
}
```

This example has one neat trick to it: when the stall condition is restarted, the duration is manually updated so that it persists on the Mon to the next turn. If a Mon does not use a stalling move on the next turn, the duration will decrease to zero at the end of the turn and the condition will end naturally.

##### Move: Counter

If a Mon uses Counter and is hit by a physical move for damage on the same turn, the Mon will hit its last attacker for double the damage.

Counter works by adding a volatile status condition to the user at the beginning of the turn instead of when the move is used. This allows us to record state on the Mon before the move is used, which is the core part of how counter works.

```json
{
  "effect": {
    "callbacks": {
      "on_before_turn": ["add_volatile: $mon $this.id"],
      "on_try_use_move": [
        "$effect_state = func_call(volatile_status_state: $user $this.id)",
        "if !$effect_state or !$effect_state.target_side or $effect_state.target_position.is_undefined:",
        ["return false"]
      ],
      "on_move_damage": [
        "$effect_state = func_call(volatile_status_state: $source $this.id)",
        "if !$effect_state:",
        ["return 0"],
        "return $effect_state.damage"
      ]
    }
  },
  "condition": {
    "duration": 1,
    "no_copy": true,
    "callbacks": {
      "on_start": ["$effect_state.damage = 0"],
      "on_redirect_target": [
        "if $move.id != counter:",
        ["return"],
        "if !$effect_state.target_side or $effect_state.target_position.is_undefined:",
        ["return"],
        "return func_call(mon_in_position: $effect_state.target_side $effect_state.target_position)"
      ],
      "on_damaging_hit": [
        "if !func_call(is_ally: $source $target) and $move.category == physical:",
        [
          "$effect_state.target_side = $source.side",
          "$effect_state.target_position = $source.position",
          "$effect_state.damage = 2 * $damage"
        ]
      ]
    }
  }
}
```

##### Move: Pursuit

Pursuit works like a normal move, but if any target on the opposing side switches out, Pursuit activates immediately and damages the Mon before the switch takes place.

Pursuit gets its own event (`BeforeSwitchOut`) that activates when any Mon switches out on the target side.

```json
{
  "effect": {
    "callbacks": {
      "on_move_base_power": [
        "if $target.being_called_back or $target.needs_switch:",
        ["return $move.base_power * 2"]
      ],
      "on_before_turn": [
        "$side = $mon.foe_side",
        "add_side_condition: $side $this.id use_target_as_source",
        "$pursuit_state = func_call(side_condition_effect_state: $side $this.id)",
        "if !$pursuit_state.sources:",
        ["$pursuit_state.sources = []"],
        "$pursuit_state.sources = func_call(append: $pursuit_state.sources $mon)"
      ],
      "on_use_move": [
        "if $target.being_called_back or $target.needs_switch:",
        ["$move.accuracy = exempt"]
      ],
      "on_try_hit": [
        "$pursuit_state = func_call(side_condition_effect_state: $target.side $this.id)",
        "if !$pursuit_state or !$pursuit_state.sources:",
        ["return"],
        "$pursuit_state.sources = func_call(remove: $pursuit_state.sources $source)"
      ]
    }
  },
  "condition": {
    "duration": 1,
    "callbacks": {
      "on_before_switch_out": [
        "$activated = false",
        "# Make a copy, since this list is mutated after Pursuit hits.",
        "$sources = $effect_state.sources",
        "foreach $source in $sources:",
        [
          "if !func_call(is_adjacent: $source $mon) or !func_call(cancel_move: $source) or $source.hp == 0:",
          ["continue"],
          "if !$activated:",
          ["$activated = true", "log_activate: with_target"],
          "do_move: $source $this.id func_call(target_location_of_mon: $source $mon) $mon"
        ]
      ]
    }
  }
}
```

##### Move: Sky Drop

Sky Drop is one of the most complex moves. On the turn its selected, the user takes the target into the air. While in the air, the target cannot act, and both Mons are invulnerable as if they used Fly. The user is locked into the move, so on the next turn, the user throws the target into the ground to receive damage, ending the move and all of its effects.

Sky Drop is so complicated, in fact, that it suffered from its own glitch in Generation V, causing it to be banned in game. If Gravity brought the user down from the sky, the target would be stuck in an immobilized state.

The complexity is boiled down to many volatile statuses being applied to two Mons at the same time, and they are all tightly coupled to one another. The user and target are in the same states, with only differences in who is attacking.

To make managing these effects easier, battler has the ability to link effects to one another inherently. When an effect ends, all effects linked to it also end.

First, Sky Drop is generalized into two effects: "Immobilizing Move" and "Immobilized."

"Immobilizing Move" is applied to a Mon that is locked into a move that immobilizes itself and its target by taking them away from the battlefield. It uses "Two Turn Move" to reuse behavior from traditional charge moves, and it applies the "Immobilized" effect to the target.

```json
{
  "name": "Immobilizing Move",
  "condition_type": "Built-in",
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": [
        "add_volatile: $target twoturnmove use_source_effect link",
        "add_volatile: $source immobilized use_target_as_source use_source_effect link"
      ],
      "on_drag_out": ["return false"],
      "on_trap_mon": ["return true"],
      "on_redirect_target": {
        "order": 1,
        "program": ["return $effect_state.source"]
      }
    }
  }
}
```

The "Immobilized" effect is applied to a Mon that is the target of an immobilizing move. It receives the volatile from the move as well, without being locked into that move. It can always be hit by the move that put it into this state, and it cannot move in this state.

```json
{
  "immobilized": {
    "name": "Immobilized",
    "condition_type": "Built-in",
    "condition": {
      "duration": 2,
      "callbacks": {
        "on_start": [
          "$effect_state.move = $source_effect.id",
          "add_volatile: $target $effect_state.move use_source_effect link"
        ],
        "on_end": ["log_end: use_effect_state_source_effect"],
        "on_drag_out": ["return false"],
        "on_trap_mon": ["return true"],
        "on_before_move": {
          "priority": 12,
          "program": ["return false"]
        },
        "on_invulnerability": {
          "order": 1,
          "program": [
            "# Allow the targeting move to hit on its second turn.",
            "if $move.id == $effect_state.move and $source == $effect_state.source:",
            ["return true"]
          ]
        }
      }
    }
  }
}
```

Notice that both the user and the target receive the volatile status associated with the move itself. In this case, both the user and target will have the "Sky Drop" effect applied to them. Let's see how this move is defined and makes use of these effects.

```json
{
  "effect": {
    "callbacks": {
      "on_use_move": [
        "if !func_call(has_volatile: $user $this.id):",
        ["$move.accuracy = exempt", "remove_move_flag: $move contact"]
      ],
      "on_try_immunity": [
        "if func_call(has_volatile: $source $this.id):",
        ["if func_call(has_type: $target flying):", ["return false"]],
        "else:",
        ["if $target.weight >= 2000:", ["return false"]]
      ],
      "on_try_hit": [
        "if func_call(has_volatile: $source $this.id):",
        [
          "# Ensure we are targeting the original target.",
          "$immobilizing_effect_state = func_call(volatile_effect_state: $source immobilizingmove)",
          "if !$immobilizing_effect_state or $target != $immobilizing_effect_state.source:",
          ["return false"],
          "remove_volatile: $source immobilizingmove"
        ],
        "else:",
        [
          "if $target.is_behind_substitute or func_call(is_ally: $source $target):",
          ["return false"],
          "log_prepare_move: $target",
          "add_volatile: $source immobilizingmove use_target_as_source",
          "return stop"
        ]
      ],
      "on_move_failed": ["remove_volatile: $user $this.id"]
    }
  },
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": ["add_volatile: $target fly link"]
    }
  }
}
```

The Sky Drop move has logic for handling the two different turns of the move. The condition simply delegates to the "Fly" effect, since Sky Drop and Fly produce the same effects on the Mon (avoiding most attacks, double damage from moves like Gust).

All in all, these three effects produce the following behavior:

1. Turn 1 - Hawlucha uses Sky Drop against Samurott.
   1. Hawlucha receives the "Immobilizing Move" volatile.
   1. Hawlucha receives the "Two Turn Move" volatile.
   1. Hawlucha receives the "Sky Drop" volatile.
   1. Hawlucha receives the "Fly" volatile.
   1. Samurott receives the "Immobilized" volatile.
   1. Samurott receives the "Sky Drop" volatile.
   1. Samurott receives the "Fly" volatile.
2. Turn 2 - Hawlucha uses Sky Drop, locked into Samurott.
   1. Hawlucha's "Immobilizing Move" volatile is removed.
   1. All linked effects are removed.

Turn 2 simply boils down to removing one effect, which cascade removes all linked effects. This has the benefit that if the move fails at any time between the move is used and the move finishes (such as if the user or target faints), all the volatile statuses from the move are cleaned up.
