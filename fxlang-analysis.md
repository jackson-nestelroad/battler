# fxlang Analysis

## Background

In the fall of 2023, I started writing a Pokémon battle engine using the Rust programming language. My goal was purely recreational: I wanted a project that allowed me to write more Rust, and I wanted the challenge of exploring how to implement such dynamic battle mechanics and interactions in a statically-compiled language. With this in mind, I started writing `battler`: a Rust-based Pokémon battle engine.

`battler` focuses on the backend logic of a Pokémon battle, producing a battle log that can be consumed by clients to understand the flow of a battle (and to build UI elements on top of it).

### Battle Mechanics

If you are unfamiliar with the games, Pokémon battles are turn-based. Players bring a team of Pokémon to battle against one another. On each turn, each player can perform an action for each of their active Pokémon. An action can involve using a move, switching out, using an item, or escaping (though there are other, more niche actions).

Pokémon moves are complex on their own. They can deal damage to one or more targets, or apply conditions to a Pokémon, side, or even the field itself. Some moves can hit multiple times, apply recoil damage, heal the user, have a random chance to apply some secondary effect, and more.

While moves are complex on their own, they are far from the only thing that impacts a battle. There are abilities, items, statuses, volatile statuses (which can stack), weather, side conditions, field conditions, and more. These effects introduce complex interactions with other effects, including moves.

This high complexity makes supporting things like 900+ moves and 180+ abilities (Generation 9 was the most recent generation at the time I started writing the battle engine) practically impossible in the core logic of a battle engine.

We need to make battle effects easy to program for different battle events and conditions.

### Separating Data from Mechanics

A guiding principle of `battler` is that battle data (e.g., moves, abilities, items) is represented completely independently of the battle engine itself.

The core battle engine is written in Rust and solely focuses on general mechanics, such as how Pokémon are represented, how various parts of the battle are modified in generic ways, and how battle actions are queued and executed.

On the other hand, the battle data itself is loaded into the battle engine on startup. Data can be represented in many formats (all the battle data schemas use `serde` for serialization). I chose JSON since it is easy to read and write.

Because of this design of separating battle data from the core battle engine, complex and highly-specialized effects cannot be "hard-coded" in the battle engine directly. Additionally, static data schemas are not adequate for capturing the vast majority of battle effects. Various parts of a move can be interrupted or short-circuited due to complex interactions, and even something as simple as a random chance and branching cannot easily be represented. Custom moves and effects will always require some sort of custom programming.

We desire the best of both worlds: structured data definitions and custom programming outside of the battle engine itself.

### Introducing: fxlang

The solution I imagined very early on was an interpreted scripting language that could be embedded directly within the battle data itself. Specifically, the JSON data for a move, ability, item, or other condition can contain the custom programming directly!

**fxlang** (short for effects language) is a JSON-based interpreted language for writing battle effect event callbacks. Effect activation works as follows:

1. The core battle engine dispatches a multitude of events throughout its processing.
2. The battle engine collects and sorts all the active effects with an event callback defined for the event.
3. The battle engine evaluates event callbacks one by one.

fxlang supports anything a simple scripting language does: variables, lists, objects, expressions, function calls, branching, and iteration.

fxlang hooks directly into the battle engine for mutating the battle. It has battle-specific variable types (e.g., Pokémon, effects, boost tables), properties for accessing state, and functions for triggering common effects (e.g., stat boosts, setting a status, changing an ability).

#### Examples

fxlang effects and conditions can be as simple or as complex as is required.

Here is the fxlang effect for the move "Pain Split":

```json
{
  "effect": {
    "callbacks": {
      "on_hit": [
        "$target_hp = $target.undynamaxed_hp",
        "$average_hp = func_call(max: 1 expr(($target_hp + $source.hp) / 2))",
        "$target_diff = $target_hp - $average_hp",
        "set_hp: $target $average_hp",
        "set_hp: $source $average_hp"
      ]
    }
  }
}
```

Here is the fxlang effect for the ability "Static":

```json
{
  "effect": {
    "callbacks": {
      "on_damaging_hit": [
        "if func_call(move_makes_contact: $move) and func_call(chance: 3 10):",
        ["set_status: $source par"]
      ]
    }
  }
}
```

Here is the fxlang effect for the "Sleep" status condition:

```json
{
  "condition": {
    "callbacks": {
      "is_asleep": ["return true"],
      "on_start": [
        "log_status: $this.name",
        "# 1-3 turns.",
        "$effect_state.total_time = func_call(random: 2 5)",
        "$effect_state.time = +$effect_state.total_time"
      ],
      "on_before_move": {
        "priority": 10,
        "program": [
          "$effect_state.time = $effect_state.time - 1",
          "if $effect_state.time <= 0:",
          ["cure_status: $user no_effect", "return"],
          "log_cant",
          "require !func_call(move_has_flag: $move sleepusable) else return",
          "return stopfail"
        ]
      },
      "on_source_try_hit": [
        "require !$move.effect_state.source_effect or $move.effect_state.source_effect.id != sleeptalk else return",
        "require func_call(move_has_flag: $move.id sleepusable) else return stop"
      ],
      "on_modify_catch_rate": ["return $catch_rate * 5/2"]
    }
  }
}
```

Here is the fxlang effect for the move "Wide Guard":

```json
{
  "hit_effect": {
    "side_condition": "wideguard"
  },
  "effect": {
    "callbacks": {
      "on_prepare_hit": ["return func_call(any_mon_will_move_this_turn)"],
      "on_try_hit_side": ["add_volatile: $source stall"]
    }
  },
  "condition": {
    "duration": 1,
    "delegates": ["condition:weakenthroughprotectionmovebase"],
    "callbacks": {
      "on_side_start": ["log_single_turn: use_source with_target"],
      "on_try_hit": [
        "require [alladjacent, alladjacentfoes] has $move.target else return",
        "if $report:",
        ["log_activate: with_target"],
        "return stop"
      ]
    }
  }
}
```

### Completing Implementation

In June 2026, after nearly three years of working on the project on and off, _all_ battle mechanics through the Generation 9 Pokémon games were implemented in `battler`. While not every single move, ability, and item is tested (especially in regard to interactions around ordering), there are around 1400 tests that cover nearly all the unique features offered by the battle engine.

In short, there is core battle engine support and battle data for:

1. Multiple battle types: Singles, Doubles, Triples, Multi.
2. Team preview.
3. All move types, abilities, items.
4. Status conditions, volatile status conditions, side conditions, slot conditions, pseudo-weathers, weathers, terrains.
5. Type changes, forme changes, ability changes, item changes, and many more volatile properties.
6. Single-player mechanics: experience, level up, move learning, escaping, affection, disobedience, bag items, catching.
7. Special mechanics: Mega Evolution, Primal Reversion, Z-Moves, Ultra Burst, Dynamax, Gigantamax, Terastallization.

## Using fxlang to Analyze Battle Effects

As a sort of celebration of this achievement, I thought it would be interesting to use the fxlang effects and conditions (implemented for all the different battle mechanics and effects) to highlight some of the most complex battle effects.

While complexity is certainly subjective, I attempted to analyze the following properties:

1. **Statement Count (Length)** - The total count of active fxlang statements across all the effect's resolved callbacks (ignoring whitespace and comments).
2. **Statement Uniqueness Score** - Measures how unique the code statements are. The fewer effects that have a given statement, the better the score. In other words, one-off statements (e.g., a unique function call) score higher than generic, highly-reused statements.
3. **Feature Uniqueness Score** - Measures the usage of rare language properties or custom functions. All variables, functions, and event callbacks are extracted.
4. **Control Flow Complexity** - The count of branching keywords in resolved statements.
5. **Callback Event Count** - The number of distinct event callbacks registered.
6. **State Mutation Count** - The number of statements that mutate `$effect_state`, which indicates stateful complexity.

These factors are used to give us a starting list. I will additionally introduce a subjectivity factor. Some effects may be preferred over others just based on the subjective knowledge of how much work it took to implement or based on how much heavy lifting the core battle engine may be doing behind the scenes.

## Battle Mechanics

Generations 6 through 9 introduce special battle mechanics, which can be activated once per battle when a Pokémon makes a move. Since there are only four generational gimmicks, we can analyze all of them.

### #4 - Mega Evolution

_Introduced: Generation 6_

<details>
<summary><i>Code</i></summary>

```
none!
```

</details>

Mega Evolution allows a Pokémon holding its respective Mega Stone to change into a more powerful forme for the duration of the battle.

Mega Evolution is essentially just a glorified forme change. In fact, it is implemented entirely within the battle engine itself (no fxlang code). The reason for this is that choosing to Mega Evolve is closely tied to player state (it is chosen in player input, validated against player state, and stored as an action on the battle queue).

Thus, Mega Evolution is the simplest mechanic: its logic for generating and validating the decision to Mega Evolve and the forme change logic are completely reusable in other parts of the battle.

Note that Primal Reversion and Ultra Burst are slight modifications of Mega Evolution.

### #3 - Dynamax

_Introduced: Generation 8_

<details>
<summary><i>Code</i></summary>

```json
{
  "condition": {
    "duration": 3,
    "no_copy": true,
    "callbacks": {
      "on_start": [
        "remove_volatile: $target minimize",
        "remove_volatile: $target substitute",
        "remove_volatile: $target torment"
      ],
      "on_upgrade_move": [
        "modify_move_type: $move",
        "$max_move_id = func_call(max_move: $user $move)",
        "require $max_move_id.is_defined else return",
        "$max_move = func_call(new_active_move: $max_move_id $user)",
        "set_upgraded_to_max_move: $max_move $move.id",
        "if $move.max_move_base_power.is_defined and !$max_move.base_power:",
        ["$max_move.base_power = $move.max_move_base_power"],
        "$max_move.category = $move.category",
        "# Move is already being used, but priority can affect other things as the move runs.",
        "$max_move.priority = $move.priority",
        "return $max_move"
      ],
      "on_source_modify_damage": [
        "if [behemothbash, behemothblade, dynamaxcannon] has $move.id:",
        ["return $damage * 2"]
      ],
      "on_add_volatile": ["if [encore, flinch, torment] has $volatile.id:", ["return false"]],
      "on_before_switch_out": ["remove_volatile: $mon $this.id"],
      "on_drag_out": ["log_block: with_target", "return stopfail"],
      "suppress_mon_item": [
        "if $mon.item.is_defined and func_call(item_has_flag: $mon.item choicelocking):",
        ["return true"]
      ],
      "on_residual": {
        "priority": -100,
        "program": []
      },
      "on_end": ["end_dynamax: $target"]
    }
  }
}
```

</details>

A Pokémon can Dynamax and use Max Moves for 3 turns or until it switches out. A Dynamaxed Pokémon has boosted HP, and Max Moves are incredibly strong with powerful secondary effects.

Dynamax is implemented almost entirely as a volatile status applied to the Pokémon. However, it did introduce several new mechanics with decent complexity:

1. Moves automatically upgrade to their equivalent Max Moves, which feature unique secondary effects.
2. The Pokémon's HP is increased by an amount determined by its Dynamax level. Many other HP-modifying effects that used a Pokémon's maximum HP had to be adjusted to use the "un-Dynamaxed maximum HP" value.
3. Gigantamax performs a forme change on top of the Dynamax volatile status.
4. Dynamax Pokémon are additionally immune to several volatile statuses and other effects (e.g., choice-locking items, force switches).

Dynamax's implementation as a volatile status is extremely elegant: the core battle engine simply applies this volatile status when Dynamax is activated, avoiding the need to hard-code interactions. Additionally, its move upgrade logic is shared with a mechanic introduced in an earlier generation, which beats it out on the complexity scale.

### #2 - Terastallization

_Introduced: Generation 9_

<details>
<summary><i>Code</i></summary>

```json
{
  "condition": {
    "callbacks": {
      "on_force_types": [
        "require $mon.terastallized.is_defined and $mon.terastallized != stellar else return",
        "return [$mon.terastallized]"
      ],
      "on_set_types": {
        "order": 1,
        "program": ["return false"]
      },
      "on_add_type": {
        "order": 1,
        "program": ["return false"]
      },
      "on_use_move": ["if $user.terastallized == stellar:", ["$move.force_stab = true"]],
      "on_modify_stab": {
        "order": 1,
        "program": [
          "require $user.terastallized.is_defined else return",
          "if $user.terastallized == stellar:",
          [
            "if !$effect_state.boosted_types:",
            ["$effect_state.boosted_types = func_call(new_object)"],
            "if $move.effect_state.stellar_boosted or !func_call(object_get: $effect_state.boosted_types $move.type.to_string):",
            [
              "if func_call(has_type_before_forced_types: $user $move.type):",
              ["$stab = 2"],
              "else:",
              ["$stab = 6/5"],
              "$move.effect_state.stellar_boosted = true",
              "$effect_state.boosted_types = func_call(object_set: $effect_state.boosted_types $move.type.to_string true)",
              "return $stab"
            ],
            "return 1"
          ],
          "else if func_call(has_type_before_forced_types: $user $user.terastallized):",
          ["return $stab * 4/3"]
        ]
      },
      "on_force_effectiveness": [
        "if $target.terastallized.is_defined and $effect.type == stellar:",
        ["return 1"]
      ]
    }
  }
}
```

</details>

Terastallization changes a Pokémon's type for the duration of the battle. A Terastallized Pokémon gains a boosted STAB multiplier for its moves.

Like Dynamax, Terastallization is implemented as an fxlang condition applied to a Pokémon. However, since Terastallization is a non-volatile property, the battle engine tracks a Pokémon's Terastallization status and applies the condition universally.

Terastallization is not overly complex on its own: it forces the Pokémon to be a specific type, prevents any type modifiers, and increases STAB. However, Terastallization _additionally_ introduces the new Stellar type (which can only be applied in the Terastallization condition). The Stellar type adds much more complexity: a Stellar-type Pokémon gains STAB on a move once per battle.

Due to being more tightly coupled within the battle engine itself and the introduction of a new type, Terastallization is viewed as more complex than Dynamax. Though the two are arguably quite close.

### #1 - Z-Moves

_Introduced: Generation 7_

<details>
<summary><i>Code</i></summary>

```json
{
  "condition": {
    "duration": 1,
    "no_copy": true,
    "callbacks": {
      "on_start": ["log_single_turn: with_target"],
      "on_upgrade_move": [
        "$z_move_id = func_call(z_move: $user $move)",
        "require $z_move_id.is_defined else return",
        "$type = $move.type",
        "modify_move_type: $move",
        "if $type != $move.type:",
        [
          "$new_z_move_id = func_call(z_move: $user $move)",
          "if $new_z_move_id.is_defined and $new_z_move_id != $z_move_id:",
          [
            "$old_z_move = func_call(get_move: $z_move_id)",
            "require $old_z_move.is_defined else return",
            "$old_z_move_active_move = func_call(new_active_move: $old_z_move.id $user)",
            "log_use_move: $old_z_move_active_move",
            "do_not_animate_last_move: $old_z_move_active_move",
            "$z_move_id = $new_z_move_id"
          ]
        ],
        "$z_move = func_call(new_active_move: $z_move_id $user)",
        "set_upgraded_to_z_move: $z_move $move.id",
        "require $user.item.is_defined else return",
        "if $move.z_move_base_power.is_defined and !$z_move.base_power:",
        ["$z_move.base_power = $move.z_move_base_power"],
        "$z_move.category = $move.category",
        "# Move is already being used, but priority can affect other things as the move runs.",
        "$z_move.priority = $move.priority",
        "return $z_move"
      ],
      "on_pre_move_effect": [
        "if $move.category == status:",
        ["add_attribute_to_last_move: zpower"],
        "if $move.z_power_boosts.is_defined:",
        ["boost: $user $move.z_power_boosts"],
        "else if $move.z_power_effect.is_defined:",
        ["activate_applying_effect: $move.z_power_effect"]
      ]
    }
  }
}
```

</details>

The most complex battle mechanic goes to Z-Moves. A Pokémon holding an eligible Z-Crystal may transform its next move into a more powerful Z-Move, once per battle.

Z-Crystal eligibility and activation is completely tracked in the battle engine. When a Pokémon prepares to use a Z-Move, the battle engine adds the "Z-Power" volatile status, which holds the logic for a) upgrading the selected move to the proper Z-Move and b) applying the Z-Power effect (for status moves).

Every single status move between Generations 1 and 7 was required to add its corresponding Z-Power effect. The sheer volume of changes that Z-Moves required across all battle data adds to their complexity.

There are several other interactions that make Z-Moves complex:

- Many move-modifying effects fail for upgraded moves.
- Z-Powers apply even if the move itself will fail early due to an immunity.
- Some Z-Crystals are species-exclusive.

Additionally, Z-Moves are the first mechanic to introduce dynamic move upgrades (which is reused by Dynamax). These interactions and novelties make Z-Moves the most complex battle mechanic to implement.

## Moves

We will look at the top 24 most complex moves.

Due to the vast array of moves, this list may be quite subjective. Some ordering between moves may be up for debate.

### #24 - Court Change

_Introduced: Generation 8_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_hit_field": [
        "$side_conditions = [mist, lightscreen, reflect, spikes, safeguard, tailwind, toxicspikes, stealthrock, waterpledge, firepledge, grasspledge, stickyweb, auroraveil, luckychant, gmaxsteelsurge, gmaxcannonade, gmaxvinelash, gmaxwildfire, gmaxvolcalith]",
        "require func_call(swap_side_conditions: $source.foe_side $source.side $side_conditions)",
        "log_activate: with_source"
      ]
    }
  }
}
```

</details>

Court Change swaps several side conditions between the two sides of the battle. The list of conditions swapped is large (screens, guards, entry hazards, etc.), and they are swapped simply by delegating to a special function provided by the battle engine.

Court Change is the only effect that swaps side conditions between two sides.

### #23 - Moves that Add Types

_Introduced: Generation 6_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_hit": ["return func_call(add_type: $target ghost)"]
    }
  }
}
```

</details>

Trick-or-Treat and Forest's Curse are two moves with a unique effect: they _add_ a type to the Pokémon. Unlike changing a Pokémon's type, adding a type preserves the Pokémon's existing typing and simply adds a volatile type on top. This allows a Pokémon to have up to three types simultaneously!

The battle engine must account for the added type property in all of its logic for representing and using a Pokémon's list of types.

### #22 - Curse

_Introduced: Generation 2_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "volatile_status": "curse"
  },
  "effect": {
    "callbacks": {
      "on_move_target_override": ["if !func_call(has_type: $mon ghost):", ["return user"]],
      "on_use_move": [
        "if !func_call(has_type: $user ghost):",
        ["$move.target = user", "set_z_power_boosts: $move func_call(boost_table: 'atk:1')"],
        "else if func_call(is_ally: $user $target):",
        ["$move.target = randomnormal"]
      ],
      "on_try_hit": [
        "if func_call(has_type: $source ghost):",
        ["$effect_state.apply_self_damage = true", "return"],
        "$effect_state.apply_self_damage = false",
        "require !func_call(has_volatile: $target $this.id)",
        "$move.hit_effect.volatile_status = undefined",
        "$move.user_effect = func_call(hit_effect)",
        "$move.user_effect.boosts = func_call(boost_table: 'spe:-1' 'atk:1' 'def:1')"
      ],
      "on_hit": [
        "if $effect_state.apply_self_damage:",
        ["direct_damage: $source expr($source.max_hp / 2)"]
      ]
    }
  },
  "condition": {
    "callbacks": {
      "on_start": ["log_start"],
      "on_residual": ["damage: $target expr($target.base_max_hp / 4)"]
    }
  }
}
```

</details>

Curse is an entirely different move depending on whether the user is a Ghost type. Ghost-type users will lose half of their maximum HP and curse the target with a volatile status. Non-Ghost types will have simple stat boosts and drops applied.

The move is complex due to its high amount of branching. Additionally, the battle engine itself must be able to represent the effective target of the move to the player for target selection. For example, in a Double battle, Ghost-type Pokémon are required to select a target when using Curse. However, if the Pokémon changes type before the move is executed, the target must be adjusted to hit the user itself.

### #21 - Disable

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "volatile_status": "disable"
  },
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "require $target.last_move.is_defined",
        "require $target.last_move.id != struggle",
        "require !$target.last_move.upgraded"
      ]
    }
  },
  "condition": {
    "duration": 5,
    "no_copy": true,
    "callbacks": {
      "on_start": [
        "# The target hasn't taken its turn yet, so this turn counts as a disabled turn.",
        "if func_call(will_move_this_turn: $target):",
        ["$effect_state.duration = $effect_state.duration - 1"],
        "require $target.last_move.is_defined",
        "# Ensure the target's last move can be disabled.",
        "foreach $move_slot in $target.move_slots:",
        ["require $move_slot.id != $target.last_move.id or $move_slot.pp != 0"],
        "if $source_effect.is_ability:",
        ["log_start: with_source_effect str('disabledmove:{}', $target.last_move.name)"],
        "else:",
        ["log_start: str('disabledmove:{}', $target.last_move.name)"],
        "$effect_state.move = $target.last_move.id"
      ],
      "on_end": ["log_end"],
      "on_before_move": {
        "priority": 7,
        "program": ["if $move.id == $effect_state.move:", ["log_cant", "return stopfail"]]
      },
      "on_disable_move": [
        "foreach $move_slot in $mon.move_slots:",
        ["if $move_slot.id == $effect_state.move:", ["disable_move: $mon $move_slot.id"]]
      ]
    }
  }
}
```

</details>

Disable temporarily prevents the target from using its last move.

The move is mostly complex because it introduced tracking a Pokémon's last move and disabling a move dynamically. Many moves in the future would build on this functionality, but it was one-of-a-kind in Generation 1.

### #20 - Encore

_Introduced: Generation 2_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "volatile_status": "encore"
  },
  "condition": {
    "duration": 3,
    "no_copy": true,
    "callbacks": {
      "on_start": [
        "require $target.last_move.is_defined",
        "$last_move_id = $target.last_move.id",
        "if $target.last_move.upgraded and $target.last_move.upgraded_base_move.is_defined:",
        ["$last_move_id = $last_move.upgraded_base_move"],
        "$last_move = func_call(get_move: $last_move_id)",
        "require $last_move.is_defined and !func_call(move_has_flag: $last_move failencore)",
        "$index = func_call(move_slot_index: $target $last_move.id)",
        "require $index.is_defined",
        "$move_slot = func_call(move_slot_at_index: $target $index)",
        "require $move_slot.is_defined and $move_slot.pp != 0",
        "$effect_state.move = $last_move.id",
        "log_start",
        "if func_call(will_move_this_turn: $target):",
        ["$effect_state.duration = $effect_state.duration + 1"]
      ],
      "on_override_move": ["if $move != $effect_state.move:", ["return $effect_state.move"]],
      "on_residual": [
        "$index = func_call(move_slot_index: $target $effect_state.move)",
        "if $index.is_defined:",
        ["$move_slot = func_call(move_slot_at_index: $target $index)"],
        "if $move_slot.is_defined and $move_slot.pp == 0:",
        ["remove_volatile: $target $this.id"]
      ],
      "on_end": ["log_end"],
      "on_disable_move": [
        "require func_call(has_move: $mon $effect_state.move) else return",
        "foreach $move_slot in $mon.move_slots:",
        ["if $move_slot.id != $effect_state.move:", ["disable_move: $mon $move_slot.id"]]
      ]
    }
  }
}
```

</details>

Encore temporarily forces the target to repeat its last move. The base move of an upgraded move is considered, and the volatile status is removed if the Pokémon no longer knows the encored move.

The logic for determining the base move is quite involved, and an additional event is required for forcing a Pokémon into its encored move when it is affected mid-turn.

### #19 - Stockpile

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "volatile_status": "stockpile"
  },
  "effect": {
    "callbacks": {
      "on_try_use_move": [
        "$stockpile_state = func_call(volatile_status_effect_state: $user $this.id)",
        "require !$stockpile_state or $stockpile_state.layers < 3"
      ]
    }
  },
  "condition": {
    "no_copy": true,
    "callbacks": {
      "on_start": [
        "$effect_state.layers = 1",
        "$effect_state.def = +0",
        "$effect_state.spd = +0",
        "log_start: str('count:{}', $effect_state.layers)",
        "$def = $target.boosts.def",
        "$spd = $target.boosts.spd",
        "boost: $target 'def:1' 'spd:1'",
        "if $def != $target.boosts.def:",
        ["$effect_state.def = -1"],
        "if $spd != $target.boosts.spd:",
        ["$effect_state.spd = -1"]
      ],
      "on_restart": [
        "require $effect_state.layers < 3",
        "$effect_state.layers = $effect_state.layers + 1",
        "log_start: str('count:{}', $effect_state.layers)",
        "$def = $target.boosts.def",
        "$spd = $target.boosts.spd",
        "boost: $target 'def:1' 'spd:1'",
        "if $def != $target.boosts.def:",
        ["$effect_state.def = $effect_state.def - 1"],
        "if $spd != $target.boosts.spd:",
        ["$effect_state.spd = $effect_state.spd - 1"],
        "return true"
      ],
      "on_end": [
        "if $effect_state.def != 0 or $effect_state.spd != 0:",
        [
          "$boosts = func_call(boost_table)",
          "$boosts.def = $effect_state.def",
          "$boosts.spd = $effect_state.spd",
          "boost: $target $boosts"
        ],
        "log_end"
      ]
    }
  }
}
```

Spit Up:

```json
{
  "effect": {
    "callbacks": {
      "on_try_use_move": ["return func_call(has_volatile: $user stockpile)"],
      "on_move_base_power": [
        "$stockpile_state = func_call(volatile_status_effect_state: $source stockpile)",
        "if !$stockpile_state:",
        ["return 0"],
        "return $stockpile_state.layers * 100"
      ],
      "on_after_move": ["remove_volatile: $user stockpile"]
    }
  }
}
```

Swallow:

```json
{
  "effect": {
    "callbacks": {
      "on_try_use_move": ["return func_call(has_volatile: $user stockpile)"],
      "on_hit": [
        "$stockpile_state = func_call(volatile_status_effect_state: $source stockpile)",
        "require $stockpile_state.is_defined",
        "if $stockpile_state.layers <= 1:",
        ["$heal_percent = 1/4"],
        "else if $stockpile_state.layers == 2:",
        ["$heal_percent = 1/2"],
        "else:",
        ["$heal_percent = 1"],
        "$heal_amount = $target.max_hp * $heal_percent",
        "require func_call(heal: $target $heal_amount) else return stop",
        "remove_volatile: $target stockpile"
      ]
    }
  }
}
```

</details>

Stockpile temporarily increases a Pokémon's defense stats, up to three times. The volatile status allows the Pokémon to use the Spit Up or Swallow move. Spit Up damages a target, powered by the number of times Stockpile was used. Likewise, Swallow heals the user for an amount based on the Stockpile count.

Stockpile is complex because it connects to two other moves. Additionally, the move's volatile status must remember the number of boosts applied by the move, so that the boosts can be reset after the volatile status is removed.

### #18 - Countering Moves

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

Counter:

```json
{
  "effect": {
    "callbacks": {
      "on_before_turn": ["add_volatile: $user $this.id"],
      "on_try_use_move": [
        "$effect_state = func_call(volatile_status_effect_state: $user $this.id)",
        "require $effect_state.is_defined",
        "require $effect_state.target_side.is_defined",
        "require $effect_state.target_position.is_defined"
      ],
      "on_move_damage": [
        "$effect_state = func_call(volatile_status_effect_state: $source $this.id)",
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
        "require $move.id == $this.id else return",
        "require $effect_state.target_side.is_defined and $effect_state.target_position.is_defined else return",
        "return func_call(mon_in_position: $effect_state.target_side $effect_state.target_position)"
      ],
      "on_damaging_hit": [
        "$category = func_call(value_from_local_data: category)",
        "require !func_call(is_ally: $source $target) else return",
        "require !$category or $move.category == $category else return",
        "$effect_state.target_side = $source.side",
        "$effect_state.target_position = $source.position",
        "$effect_state.damage = 2 * $damage"
      ]
    },
    "local_data": {
      "values": {
        "category": "physical"
      }
    }
  }
}
```

</details>

Counter and Mirror Coat have the same effect for physical and special moves respectively. During the turn, they save the last damage taken from an attacker. Once the Pokémon uses the move, they hit their attacker for double the damage.

Counter requires a special event callback that begins the effect at the start of the turn; the move must also properly store attacker data for target redirection and damage calculation.

### #17 - Transform

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_hit": ["return func_call(transform_into: $source $target)"]
    }
  }
}
```

</details>

Transform, as expected, transforms the user into the target. The transformation logic is entirely within the battle engine. Many things are copied: the target's types, stats, ability, moves, species, boosts, etc.

### #16 - Sketch

_Introduced: Generation 2_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_hit": [
        "$last_move = $target.last_move",
        "require $last_move.is_defined",
        "require !$source.transformed",
        "require !func_call(has_move: $source $last_move.id)",
        "require !func_call(move_has_flag: $last_move nosketch) and !$last_move.upgraded",
        "$sketch_index = func_call(move_slot_index: $source $this.id)",
        "overwrite_move_slot: $source $sketch_index func_call(move_slot: $last_move) override_base_slot",
        "log_activate: with_target use_source str('newmove:{}', $last_move.name)"
      ]
    }
  }
}
```

</details>

Sketch overwrites itself with the target's last move. The effect is permanent.

Sketch on its own is not the most complex, but it is incredibly unique. It relies on the core battle engine supporting overwriting a move slot.

### #15 - Item Taking Moves

_Introduced: Generation 2_

<details>
<summary><i>Code</i></summary>

Thief:

```json
{
  "effect": {
    "callbacks": {
      "on_after_hit": [
        "require !$source.item else return",
        "require !!assign($item = func_call(take_item: $target)) else return",
        "require func_call(set_item: $source $item) else return"
      ]
    }
  }
}
```

Trick:

```json
{
  "effect": {
    "callbacks": {
      "on_try_immunity": ["return !func_call(has_ability: $target stickyhold)"],
      "on_try_hit": [
        "# No items.",
        "require $source.item.is_defined or $target.item.is_defined",
        "# Check that we can take and give items.",
        "if $target.item.is_defined:",
        [
          "require !!assign($target_item = func_call(take_item: $target dry_run)) and func_call(set_item: $source $target_item dry_run)"
        ],
        "if $source.item.is_defined:",
        [
          "require !!assign($source_item = func_call(take_item: $source dry_run)) and func_call(set_item: $target $source_item dry_run)"
        ]
      ],
      "on_hit": [
        "$target_item = func_call(take_item: $target)",
        "$source_item = func_call(take_item: $source)",
        "$success = false",
        "if $target_item.is_true:",
        ["set_item: $source $target_item", "$success = true"],
        "if $source_item.is_true:",
        ["set_item: $target $source_item", "$success = true"],
        "return $success"
      ]
    }
  }
}
```

Knock Off:

```json
{
  "effect": {
    "callbacks": {
      "on_move_base_power": [
        "require !!func_call(take_item: $target dry_run) else return",
        "return $move.base_power * 3/2"
      ],
      "on_after_hit": ["require $source.hp != 0 else return", "take_item: $target"]
    }
  }
}
```

</details>

Several moves take the target's held item. The added complexity in these moves is that each move must first check that the item can be taken in the first place, with a 'dry run'. Some items simply cannot be taken, such as Mega Stones and Z-Crystals on their respective species, or items held by a Pokémon with the Sticky Hold ability.

### #14 - Fling

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_prepare_hit": [
        "# User has an item that is not suppressed.",
        "$item = $user.effective_item",
        "require $item.is_defined",
        "# Item can be flung.",
        "$data = func_call(special_item_data: $item)",
        "require $data.is_defined and $data.fling.is_defined",
        "# User can lose its item.",
        "require !!func_call(take_item: $user dry_run)",
        "# Modify move according to the item being flung.",
        "$effect_state.item = $item.id",
        "$effect_state.use_item = $data.fling.use_item",
        "log_activate: with_target str('item:{}', $item.name)",
        "$move.base_power = $data.fling.power",
        "$move.hit_effect = $data.fling.hit_effect",
        "# Use a volatile to remove the user's item after the move hits.",
        "# This ensures the item is consumed even if the move fails.",
        "add_volatile: $user $this.id"
      ],
      "on_hit": [
        "if func_call(item_has_flag: $effect_state.item berry):",
        ["eat_given_item: $target $effect_state.item"],
        "else if $effect_state.use_item:",
        ["use_given_item: $target $effect_state.item"]
      ]
    }
  },
  "condition": {
    "callbacks": {
      "on_update": [
        "# Remove the user's item.",
        "discard_item: $effect_state.target",
        "remove_volatile: $effect_state.target $this.id"
      ]
    }
  }
}
```

</details>

Fling attacks the target with the user's item. Items can have a variable base power and hit effect (e.g., flinging a Flame Orb burns the target).

Flung items can also apply the effect of the item as if it was used by the Pokémon. The same applies for berries (the target consumes the berry).

The Fling data that must be attached to a multitude of items increases the complexity of this move.

### #13 - Moves with a Charging Turn

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

Fly:

```json
{
  "effect": {
    "delegates": ["condition:chargemovebase"]
  },
  "condition": {
    "duration": 2,
    "callbacks": {
      "is_grounded": ["return false"],
      "is_semi_invulnerable": ["return true"],
      "on_invulnerability": [
        "require !([gust, twister, skyuppercut, thunder, hurricane, smackdown, thousandarrows] has $move.id) else return",
        "return false"
      ],
      "on_source_modify_damage": ["if [gust, twister] has $move.id:", ["return $damage * 2"]]
    }
  }
}
```

Solar Beam:

```json
{
  "effect": {
    "delegates": ["condition:chargemovebase"],
    "callbacks": {
      "on_charge_move": [
        "$weather = func_call(effective_weather: $user)",
        "require !$weather or !$weather.is_sunny"
      ],
      "on_move_base_power": [
        "$weak_weathers = [rainweather, heavyrainweather, sandstormweather, hailweather, snowweather]",
        "if $weak_weathers has func_call(effective_weather: $source):",
        ["return $move.base_power * 1/2"]
      ]
    }
  }
}
```

Charge Move Base:

```json
{
  "condition": {
    "callbacks": {
      "on_try_use_move": [
        "require !func_call(remove_volatile: $user $this.id) else return",
        "log_prepare_move",
        "run_event: BeforeChargeMove",
        "$charge_move = func_call(run_event_on_move: ChargeMove)",
        "if $charge_move.is_defined and !$charge_move:",
        ["do_not_animate_last_move", "log_animate_move: $move $target", "return"],
        "if !func_call(run_event: ChargeMove):",
        ["do_not_animate_last_move", "log_animate_move: $move $target", "return"],
        "add_volatile: $user twoturnmove link",
        "return stop"
      ]
    }
  }
}
```

Two Turn Move:

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
        "if $source_effect.is_move and $source_effect.effect_state.source_effect.is_defined and $source_effect.target != user:",
        [
          "$need_new_target = false",
          "if !$target.last_target_location:",
          ["# No target selected for the last move.", "$need_new_target = true"],
          "else:",
          [
            "# The last target selected no longer exists or fainted.",
            "$last_target = func_call(mon_at_target_location: $target $target.last_target_location)",
            "$need_new_target = expr(!$last_target or !$last_target.active)"
          ],
          "if $need_new_target:",
          [
            "# Choose a random target for the move being used.",
            "$move_data = func_call(get_move: $effect_state.move)",
            "$target.last_target_location = func_call(target_location_of_mon: $target func_call(random_target: $target $move_data.target))"
          ]
        ],
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

</details>

Most generations have introduced one or more moves with a charging turn. Fly, Solar Beam, Dig, Shadow Force, and Electro Shot are all examples of a move with a charging turn.

Moves with a charging turn follow this common structure:

1. The Pokémon uses the move.
2. On the first turn, the move begins charging.
3. Some effect may apply to the Pokémon on the charge turn.
4. The charge turn may be skipped under some precondition.
5. During the charge turn, the Pokémon receives a volatile status condition for the move itself.
6. The Pokémon is locked into the same move for the next turn.
7. On the second turn, the volatile status condition is removed, and the move is executed.

The flow above is highly generic and satisfies several types of two-turn moves:

- Fly sends the user up in the air. The Pokémon is semi-invulnerable, though it takes additional damage from specific moves (e.g., Thunder).
- Solar Beam skips the charge turn in sunny weather.
- Skull Bash boosts the user's defense stat on the charge turn.

The code above implements this flow by using the move's volatile status as a tracker to determine if the move can execute. If the Pokémon does not have the move's volatile status, it enters the charge turn and receives the volatile status. If the Pokémon does have the move's volatile status, it executes the move.

Additional complexity is introduced for running events during the charge turn (e.g., Cramorant with Gulp Missile must change formes when using Dive) and retargeting (moves that call other moves may not have the correct target type).

### #12 - Protect

_Introduced: Generation 2_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "volatile_status": "protect"
  },
  "effect": {
    "delegates": ["condition:protectmovebase"]
  },
  "condition": {
    "delegates": ["condition:protectmoveconditionbase"]
  }
}
```

Protect Move Base:

```json
{
  "condition": {
    "callbacks": {
      "on_prepare_hit": [
        "return func_call(any_mon_will_move_this_turn) and func_call(run_event_for_mon: StallMove)"
      ],
      "on_hit": ["add_volatile: $target stall"]
    }
  }
}
```

Protect Move Condition Base:

```json
{
  "condition": {
    "duration": 1,
    "delegates": ["condition:weakenthroughprotectionmovebase"],
    "callbacks": {
      "on_start": ["log_single_turn: with_target", "add_volatile: $target $source_effect.id link"],
      "on_try_hit": {
        "priority": 3,
        "program": [
          "require func_call(move_has_flag: $move protect) else return",
          "if $report:",
          ["log_activate: with_target"],
          "activate_applying_effect: $this no_forward",
          "if $effect_state.source_effect.is_defined:",
          ["activate_applying_effect: $effect_state.source_effect no_forward"],
          "return stop"
        ]
      },
      "on_hit": [
        "if $move.upgraded:",
        [
          "activate_applying_effect: $this no_forward",
          "if $effect_state.source_effect.is_defined:",
          ["activate_applying_effect: $effect_state.source_effect no_forward"]
        ]
      ]
    }
  }
}
```

Stall:

```json
{
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": ["$effect_state.counter = 3"],
      "on_restart": [
        "if $effect_state.counter < 729:",
        ["$effect_state.counter = $effect_state.counter * 3"],
        "$effect_state.duration = 2",
        "return true"
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

</details>

Protect completely protects the user from all effects of moves during the turn. It is a critical part of any battle.

Protect has several variants that apply an additional effect when a Pokémon hits into the protection. These moves apply both the Protect volatile status and the volatile status of the move itself (which is activated by the Protect condition). For example, when an attack makes contact, Spiky Shield damages, Baneful Bunker poisons, and Burning Bulwark burns.

Protect is considered quite complex due to several unique features:

- The chance of Protect failing on each subsequent use is multiplied by one-third. Other moves share this same counter, so it is captured in the "Stall" volatile status.
- The Protect volatile status applies the protection. When an attacker hits into the protection, it activates the source effect (in order to provide additional effects against the attacker).
- Other moves can interact with Protect differently. Moves such as Feint break Protect entirely. Z-Moves and Max Moves hit through Protect but for only 25% of their intended damage.

### #11 - Moves that Call Other Moves

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

Mirror Move:

```json
{
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "$last_move = $target.last_move",
        "require $last_move.is_defined",
        "require func_call(move_has_flag: $last_move mirror)",
        "require $last_move.callable",
        "use_move: $source $last_move.id $target",
        "return skip"
      ]
    }
  }
}
```

Assist:

```json
{
  "effect": {
    "callbacks": {
      "on_hit": [
        "$moves = []",
        "foreach $mon in func_call(all_mons_on_side: $target.side):",
        [
          "if $mon == $target:",
          ["continue"],
          "foreach $move_slot in $mon.move_slots:",
          [
            "$usable_move = func_call(get_move: $move_slot.id)",
            "if !$usable_move or func_call(move_has_flag: $usable_move noassist) or !$usable_move.callable:",
            ["continue"],
            "$moves = func_call(append: $moves $usable_move.id)"
          ]
        ],
        "$random_move = func_call(sample: $moves)",
        "require $random_move.is_defined",
        "use_move: $target $random_move"
      ]
    }
  }
}
```

Snatch:

```json
{
  "hit_effect": {
    "volatile_status": "snatch"
  },
  "z_move": {
    "boosts": {
      "spe": 2
    }
  },
  "condition": {
    "duration": 1,
    "callbacks": {
      "on_start": ["log_single_turn: with_target"],
      "on_any_prepare_hit": [
        "require func_call(move_has_flag: $move snatch) else return",
        "require !$move.upgraded else return",
        "require !$move.effect_state.source_effect or $move.effect_state.source_effect.id != $this.id else return",
        "$snatch_user = $effect_state.source",
        "log_activate: with_target use_effect_state_source",
        "remove_volatile: $snatch_user $this.id",
        "use_move: $snatch_user $move.id use_effect_as_source_effect",
        "return stop"
      ]
    }
  }
}
```

Magic Coat:

```json
{
  "hit_effect": {
    "volatile_status": "magiccoat"
  },
  "z_move": {
    "boosts": {
      "spd": 2
    }
  },
  "condition": {
    "duration": 1,
    "callbacks": {
      "on_start": ["log_single_turn: with_target"],
      "on_try_hit": [
        "require $target != $source else return",
        "require !$move.effect_state.reflected else return",
        "require func_call(move_has_flag: $move reflectable) else return",
        "log_activate: with_target",
        "$reflected_move = func_call(clone_active_move: $move $target)",
        "$reflected_move.effect_state.reflected = true",
        "use_active_move: $target $reflected_move $source use_effect_as_source_effect",
        "$move.force_try_hit_result = stop",
        "return stop"
      ],
      "on_side_try_hit_side": [
        "require $source.side != $side else return",
        "require !$move.effect_state.reflected else return",
        "require func_call(move_has_flag: $move reflectable) else return",
        "log_activate: with_target use_effect_state_target",
        "$reflected_move = func_call(clone_active_move: $move $effect_state.target)",
        "$reflected_move.effect_state.reflected = true",
        "use_active_move: $effect_state.target $reflected_move $source use_effect_as_source_effect",
        "return stop"
      ]
    }
  }
}
```

Instruct:

```json
{
  "effect": {
    "callbacks": {
      "on_hit": [
        "$last_move = $target.last_move",
        "require $last_move.is_defined and !$target.dynamaxed",
        "$index = func_call(move_slot_index: $target $last_move.id)",
        "require $index.is_defined",
        "$move_slot = func_call(move_slot_at_index: $target $index)",
        "require $move_slot.is_defined and $move_slot.pp != 0",
        "require !$last_move.upgraded",
        "require !func_call(move_has_flag: $last_move failinstruct)",
        "require !func_call(move_has_flag: $last_move charge)",
        "require !func_call(move_has_flag: $last_move recharge)",
        "require !func_call(has_volatile: $target windingup)",
        "log_single_turn: with_target $target with_source",
        "$action_id = func_call(add_move_action: $target $last_move.id $target.last_target_location)",
        "prioritize_move: $target $action_id $this $source"
      ]
    }
  }
}
```

</details>

Several moves can call _other_ moves. We group these moves together because the ability to use a move from another move is shared among all of them, but the actual logic in the battle engine to do so is quite complex and has several implications for how the move execution flow works.

Internally, the battle engine distinguishes between "inactive" and "active" moves. An inactive move is the static data for the move, referenced by ID. Since the exact properties of a move can be highly dynamic based on who uses the move and when it is used, an active move makes a copy of the static move data while a Pokémon uses it. Thus, an active move is a dynamic instance of a move, referenced by a unique numerical handle internally.

There are actually three functions for running a move directly from an event callback:

- `use_active_move` - Uses a move already registered as an active move. Useful when the event callback needs to modify something about the active move prior to using it.
- `use_move` - Uses a move by ID. Effectively registers the active move, then calls `use_active_move`.
- `do_move` - Executes a selected move by ID, as if the user selected it. Things like move overrides, target selection, PP deductions, and more are handled here.

Allowing moves to call other moves requires careful logic organization, which continually had to be refined as more moves and features were added to the battle engine.

Instruct works a bit differently: it adds a new move action to the battle queue entirely and forces it to execute next. This approach is used because Instruct orders _another_ Pokémon to use a move, rather than having the user perform the move itself. Regardless, it is called out here for its similarity.

While several moves utilize this feature to call other moves, they all rely on the core battle engine doing all the heavy lifting to represent "moves within moves" properly.

### #10 - Future-Using Moves

_Introduced: Generation 2_

<details>
<summary><i>Code</i></summary>

Future Sight:

```json
{
  "effect": {
    "callbacks": {
      "on_ignore_immunity": ["return true"],
      "on_try_move": [
        "require func_call(add_slot_condition: $target.side $target.position futuremove use_target_as_source)",
        "return stop"
      ]
    },
    "local_data": {
      "moves": {
        "futuremove": {
          "name": "Future Sight",
          "category": "Special",
          "primary_type": "Psychic",
          "accuracy": 100,
          "base_power": 120,
          "target": "Scripted",
          "flags": ["Future"]
        }
      }
    }
  }
}
```

Future Move:

```json
{
  "condition": {
    "duration": 3,
    "callbacks": {
      "on_residual": {
        "order": 3
      },
      "on_slot_start": ["log_start: use_effect_state_source_effect with_source"],
      "on_slot_end": [
        "$move = func_call(new_active_move_from_local_data: $effect_state.source_effect $this.id $effect_state.source)",
        "$target = func_call(mon_in_position: $side $slot)",
        "require $target.is_defined else return",
        "require $target.active else return",
        "require $target != $effect_state.source else return",
        "$effect_state.target = $target",
        "log_end: with_target use_effect_state_target use_effect_state_source_effect with_source use_effect_state_source_as_source",
        "remove_volatile: $target protect",
        "remove_volatile: $target endure",
        "use_active_move: $effect_state.source $move $target indirect"
      ]
    }
  }
}
```

</details>

Future Sight and Doom Desire are the only two moves that are considered "future" moves: they do nothing when used but activate two turns later.

There are effectively two versions of the same move: the one that starts the move and the move that activates in the future. We achieve this with the following:

1. The move itself simply adds a slot condition for the move to activate in the future.
2. When the slot condition ends, it executes the active move on the slot. The active move data is created from the "local data" of the move.

The local data construct allows an effect to define arbitrary, dynamic data for event callbacks to reference. For instance, Future Sight defines an alternative version of itself with the correct base power and without the future-move event callbacks. This mechanism avoids the need to add branching to all the event callbacks of the original move.

These moves are even more complex when you consider the fact that the original user of the move may no longer be on the field at the time the move is executed: this means the move can technically be used by an inactive Pokémon!

Future-using moves build on the previous category of "moves that call other moves," so they rank as slightly more complex due to the additional logic needed to support them.

### #9 - Bide

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "volatile_status": "bide"
  },
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
        "$move = func_call(new_active_move_from_local_data: $this $this.id $user)",
        "$move.damage = expr($effect_state.total_damage * 2)",
        "# Remove this volatile effect before using the new move, or else this callback gets triggered endlessly.",
        "remove_volatile: $user $this.id",
        "use_active_move: $user $move $target no_source_effect",
        "# Since we used the local Bide, we can exit this move early.",
        "return stop"
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
          "advanced_targeting": {
            "no_random_target": true
          },
          "flags": ["Contact", "Protect"],
          "effect": {
            "callbacks": {
              "on_ignore_immunity": ["return true"],
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

</details>

Bide is sort of like a mix between a future-using move and a countering move. For two turns, the Pokémon stores up energy, recording all damage taken. On the third turn, the Pokémon releases an attack into its last attacker for twice the damage it received during the preparation period.

Bide uses the same locally-defined move mechanism that future-using moves use, though the local Bide version has much more going on: its target and event callbacks are all different. Bide is actually the reason the local data construct exists in the first place!

### #8 - Dragon Darts

_Introduced: Generation 8_

<details>
<summary><i>Code</i></summary>

```
none!
```

</details>

Dragon Darts deals two hits with a unique targeting strategy: the first hit is applied to the selected target, and the second hit is applied to another target. However, for both hits, if either target would take no damage for nearly any reason (e.g., immunity, protection, invulnerability, accuracy), the hit is applied to the other Pokémon (if possible). This smart targeting technique is exclusive to Dragon Darts.

Dragon Darts has no fxlang code, because the targeting is handled entirely by the core battle engine. The battle engine pretends the move will hit _all_ foes in the preparation stage, then right before it applies the move's hit effect, it iteratively applies hits on a single target.

Dragon Darts only reports that a move failed on the last target it checks. That means that logs for misses, immunities, and failures must only be logged when indicated by the battle engine. Specifically, all non-move `TryHit` events must check a special `$report` variable before logging anything.

The reliance on a unique targeting strategy, built completely within the core battle engine, is what makes Dragon Darts so complex compared to other moves.

### #7 - Skill Swap

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "volatile_status": "swapabilities"
  }
}
```

Swap Abilities:

```json
{
  "condition": {
    "duration": 0,
    "callbacks": {
      "on_start": [
        "$target_ability = $target.ability",
        "$source_ability = $source.ability",
        "require !func_call(ability_has_flag: $target_ability permanent)",
        "require !func_call(ability_has_flag: $source_ability permanent)",
        "require !func_call(ability_has_flag: $target_ability noskillswap)",
        "require !func_call(ability_has_flag: $source_ability noskillswap)",
        "require !$target.dynamaxed",
        "require func_call(set_ability: $source $target_ability use_source_effect no_forward dry_run)",
        "require func_call(set_ability: $target $source_ability use_source_effect no_forward dry_run)",
        "$target_ability = $target.ability",
        "$source_ability = $source.ability",
        "log_activate: with_target with_source use_source_effect",
        "$allies = func_call(is_ally: $target $source)",
        "if $allies:",
        [
          "set_ability: $source $target_ability use_source_effect no_forward silent",
          "set_ability: $target $source_ability use_source_effect no_forward silent"
        ],
        "else:",
        [
          "set_ability: $source $target_ability use_source_effect no_forward",
          "set_ability: $target $source_ability use_source_effect no_forward"
        ]
      ]
    }
  }
}
```

</details>

Skill Swap swaps the abilities between the user and target. Some abilities are permanent or may simply disallow Skill Swap from taking an effect altogether.

First, the move checks that the abilities _can_ be swapped with a 'dry run'. For example, the Ability Shield item prevents the holder's ability from changing. If the dry runs succeed, the two ability changes are applied. As an additional layer of complexity, the abilities are only announced if the move is used on a foe: Skill Swap between allies does not reveal the abilities.

### #6 - Ability and Item Suppressing Moves

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

Gastro Acid:

```json
{
  "hit_effect": {
    "volatile_status": "gastroacid"
  },
  "effect": {
    "callbacks": {
      "on_try_hit": ["require !func_call(ability_has_flag: $target.ability permanent)"]
    }
  },
  "condition": {
    "callbacks": {
      "suppress_mon_ability": ["if $effect_state.started:", ["return true"]],
      "on_start": [
        "if $target.can_suppress_ability:",
        ["end_ability: use_effect_state_source_as_source"]
      ],
      "on_copy_volatile": ["require !func_call(ability_has_flag: $target.ability permanent)"],
      "on_end": [
        "# This volatile can only end if Mon is switched out, so this never happens.",
        "if $target.being_called_back:",
        ["start_ability: silent"]
      ]
    }
  }
}
```

Embargo:

```json
{
  "hit_effect": {
    "volatile_status": "embargo"
  },
  "condition": {
    "duration": 5,
    "callbacks": {
      "suppress_mon_item": ["if $effect_state.started:", ["return true"]],
      "on_prevent_used_items": ["return true"],
      "on_start": ["log_start", "if $target.can_suppress_item:", ["end_item: silent"]],
      "on_end": ["log_end", "start_item: silent"]
    }
  }
}
```

</details>

Starting in Generation 4, a Pokémon's ability and held item could be completely suppressed. Gastro Acid suppresses all effects of the target's ability, though the Pokémon still possesses the ability. Embargo suppresses all effects of the target's item, though the Pokémon still holds the item.

Suppression is implemented directly in the battle engine. If a Pokémon's ability or item is found to be suppressed (via some dedicated event callback), it is excluded when generating the list of event callbacks to run for a triggering event. Since suppression is activated by its own event, checking for suppression recursively evaluates events, creating a circular dependency and a risk of infinite recursion. The battle engine avoids checking for suppression for a certain set of "low-level" events (e.g., suppression events).

Additionally, the battle engine itself can significantly slow down performance-wise with so many events running all the time. State events, including any effect suppression, are cached by the battle engine. State caches are cleared any time a new effect is added to the battle.

When an effect is suppressed, the "end" event runs in order to signal that the effect is no longer active. Later, if the suppression ends, the effect must be started up again with the "start" event.

Suppression is one of the most complex things supported by the battle engine. Its implementation stretches across many of the foundational pieces of the engine, and nearly all other effects must carefully consider if suppression should or should not be respected. For example, the move Trick honors ability suppression (if the target has the suppressed Sticky Hold ability, their item can be stolen) while a move like Skill Swap does not honor suppression (abilities can always be swapped).

### #5 - Revival Blessing

_Introduced: Generation 9_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "slot_condition": "revivalblessing"
  },
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "foreach $mon in func_call(all_mons_in_party: $target.player):",
        ["require !$mon.fainted else return"],
        "return false"
      ]
    }
  },
  "condition": {
    "callbacks": {
      "on_slot_start": ["request_mon_selection: $source revive"],
      "on_select": [
        "revive: $selected expr($selected.max_hp / 2)",
        "remove_slot_condition: $mon.side $mon.position $this.id"
      ]
    }
  }
}
```

</details>

Revival Blessing revives a fainted Pokémon of the user's choice. It introduces a completely new action: "selecting" a Pokémon purely for the purpose of the move.

The move is implemented with a slot condition. When it starts, it triggers the player to select a Pokémon. The logic here is handled by the battle engine with a new request type that halts the battle until the selection is made.

On selection, the slot condition revives the selected Pokémon, as if a Revive item was used.

The revival part of the move is also built into the battle engine. Revival Blessing is the first time that a Pokémon can be revived without using a bag item (which is typically only reserved for single-player battles). Had revival not been implemented for the bag item, this mechanic would also be completely new to the engine.

### #4 - Stomping Tantrum

_Introduced: Generation 7_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_move_base_power": ["if $source.move_last_turn_failed:", ["return $move.base_power * 2"]]
    }
  }
}
```

</details>

Stomping Tantrum looks extremely simple: it doubles in power if the user's last move failed.

However, actually determining if the last move failed is extremely complex. For example,

- Hitting into protect does not count.
- Hitting into immunity does count.
- Recharge turns do not count.
- Using a move with no effect does not count.
- Fly being interrupted by Smack Down does not count.
- Failing to use a move due to Gravity does count.
- Failing to heal at full health does not count.
- Failing to Rest due to already being asleep or at full health does count.
- Failing to Rest due to failing to fall asleep (e.g., Electric Terrain) does not count.

All this to say, complex moves can generally decide when a condition that causes a move to stop indicates a failure or not. And Stomping Tantrum allows the consistency of these decisions to be put under test directly.

fxlang uses the `EventResult` type to capture this complexity. Many functions and event callbacks return this type to control how failures are reported and recorded in the move execution flow:

- `false` - The move fails immediately. Do not animate. Report failure. Treat as failure.
- `stopfail` - Stop the move. Do not animate. Do not report failure. Treat as failure.
- `stopreportfail` - Stop the move. Do not animate. Report failure. Treat as skipped.
- `stop` - Stop the move. Do not animate. Do not report failure. Treat as skipped.
- `skip` - Skip the move. Animate. Do not report failure. Treat as skipped.
- `true` - Continue the move. Animate. Do not report failure. Treat as success.

As a rule of thumb:

- `false` is used for a true failure, where the "But it failed!" message should appear.
- `stopfail` is used for a failure that already had a message reported, such as an immunity granted by an ability.
- `stop` is used to stop a move without marking it as a failure.

Stomping Tantrum requires all core battle actions and event callbacks to consider how they fail, which makes it one of the most far-reaching moves in the engine.

Here are some moves, in no particular order, that did not make the list but are interesting nonetheless:

1. Ally Switch (Gen 5) - Swaps the position of two Pokémon. Increasing chance of failing between turns (same as Protect but a different counter).
2. Baton Pass (Gen 2) - Switches the user out, passing copyable volatile statuses to the incoming Pokémon.
3. Beat Up (Gen 2) - Hits once for each unfainted Pokémon in the player's party. The base power of each hit is determined by the Pokémon's raw attack stat.
4. Camouflage (Gen 3) - Sets the user's type based on the battle environment. Terrain and the direct field environment (e.g., in grass, on water, in a cave) affect the type selected.
5. Conversion 2 (Gen 2) - Sets the user's type to a random type that will resist (or be immune to) the target's last move.
6. Destiny Bond (Gen 2) / Grudge (Gen 3) - Applies a volatile status until the user makes another move, applying an effect if the Pokémon faints while the volatile status is active. Destiny Bond causes the attacker to faint; Grudge drains the PP of the attacker's last move.
7. Focus Punch (Gen 3) - Winds up at the start of turn. The move fails if the user is hit before it can use the move.
8. Freezy Frost (Gen 7) - Removes all stat boosts from all Pokémon on the field.
9. Gravity (Gen 4) - Applies a pseudo-weather that grounds Pokémon, increases accuracy, and disables specific moves.
10. Imprison (Gen 3) - Prevents foes from using any move that the user also knows.
11. Perish Song (Gen 2) - Applies a volatile status to all affected Pokémon, which starts a four-turn timer that causes the Pokémon to faint.
12. Reflect Type (Gen 5) - Changes the user's type to match the type of the target, including any added type.
13. Rest (Gen 1) - Fully heals the user while forcing them to sleep for three turns.
14. Rollout (Gen 2) - Increases the move's base power for each subsequent, successful hit.
15. Secret Power (Gen 3) - Changes the move's secondary effect based on the terrain and battle environment.
16. Shell Side Arm (Gen 8) - Calculates the move's base damage as both a physical and special move to determine the move's category.
17. Shell Trap (Gen 7) - Uses the move immediately after being hit by a physical move. Otherwise, the move fails.
18. Smack Down (Gen 5) - Knocks the target down, removing several volatile statuses and grounding the target.
19. Spectral Thief (Gen 7) - Steals the target's positive stat boosts before hitting for damage.
20. Spikes (Gen 2) / Toxic Spikes (Gen 4) - Creates a multi-layer entry hazard that damages (or poisons) foes on switch in.
21. Telekinesis (Gen 5) - Lifts the target into the air, marking them as "un-grounded." Certain species (e.g., Diglett, Dugtrio, Mega Gengar) are immune.
22. Topsy-Turvy (Gen 6) - Inverts the target's boosts.
23. Trump Card (Gen 4) - Increases base power based on the amount of PP left on the move.
24. Uproar (Gen 3) - Causes the user to continually make an uproar over three turns, preventing all Pokémon from falling asleep.
25. Wonder Room (Gen 5) - Swaps the Defense and Special Defense stats in the base calculation for the entire field.

### #3 - Pursuit

_Introduced: Generation 2_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_move_base_power": [
        "if $target.being_called_back or $target.needs_switch:",
        ["return $move.base_power * 2"]
      ],
      "on_before_turn": [
        "$side = $user.foe_side",
        "add_side_condition: $side $this.id",
        "$pursuit_state = func_call(side_condition_effect_state: $side $this.id)",
        "if !$pursuit_state.sources:",
        ["$pursuit_state.sources = []"],
        "$pursuit_state.sources = func_call(append: $pursuit_state.sources $user)"
      ],
      "on_use_move": [
        "if $target.being_called_back or $target.needs_switch:",
        ["$move.accuracy = exempt"]
      ],
      "on_try_hit": [
        "$pursuit_state = func_call(side_condition_effect_state: $target.side $this.id)",
        "require $pursuit_state.is_defined and $pursuit_state.sources.is_defined else return",
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

</details>

Pursuit significantly increases the complexity of switch actions by waiting for a Pokémon to switch out. Before the switch is executed, the move is used to damage the target, potentially interrupting the switch entirely by knocking the target out.

Pursuit is interesting because it is unique: no other move works in this manner. The fxlang code for this move is some of the most non-straightforward, technical code out of all battle effects.

### #2 - Sky Drop

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

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
        ["require !func_call(has_type: $target flying)"],
        "else:",
        ["require $target.weight < 2000"]
      ],
      "on_try_hit": [
        "if func_call(has_volatile: $source $this.id):",
        [
          "# Ensure we are targeting the original target.",
          "$immobilizing_effect_state = func_call(volatile_status_effect_state: $source immobilizingmove)",
          "require $immobilizing_effect_state.is_defined and $target == $immobilizing_effect_state.source",
          "remove_volatile: $source immobilizingmove"
        ],
        "else:",
        [
          "require !$target.is_behind_substitute and !func_call(is_ally: $source $target)",
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
      "is_away_from_field": ["return true"],
      "on_start": ["add_volatile: $target fly link"]
    }
  }
}
```

Immobilizing Move:

```json
{
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": [
        "add_volatile: $target twoturnmove use_source_effect link",
        "add_volatile: $source immobilized use_source_effect link"
      ],
      "on_drag_out": ["return false"],
      "on_trap_mon": {
        "order": 1,
        "program": ["return true"]
      },
      "on_redirect_target": {
        "order": 1,
        "program": ["return $effect_state.source"]
      }
    }
  }
}
```

Immobilized:

```json
{
  "condition": {
    "duration": 2,
    "callbacks": {
      "on_start": [
        "$effect_state.move = $source_effect.id",
        "add_volatile: $target $effect_state.move use_source_effect link"
      ],
      "on_end": ["log_end: use_effect_state_source_effect"],
      "on_drag_out": ["return false"],
      "on_trap_mon": {
        "order": 1,
        "program": ["return true"]
      },
      "on_before_move": {
        "priority": 12,
        "program": ["return false"]
      },
      "on_invulnerability": {
        "order": 2,
        "program": [
          "# Allow the targeting move to hit on its second turn.",
          "if $move.id == $effect_state.move and $source == $effect_state.source:",
          ["return true"]
        ]
      }
    }
  }
}
```

</details>

When a Pokémon uses Sky Drop, it takes itself and the target into the air. While in the air, the target cannot act and both Pokémon are invulnerable (as if they used Fly). The user is locked into the move, so on the next turn, they throw the target into the ground to deal damage, ending the move and all of its side effects.

Sky Drop is so complex that it suffered from a critical glitch in Generation V that caused it to be banned in online matches. The complexity comes from the large number of tightly-coupled effects being applied to two Pokémon at the same time. The user and target are put in the same state; the only difference being who is attacking.

Sky Drop uses two unique volatile statuses: "immobilizing move" and "immobilized." The former applies to the user while the latter applies to both Pokémon. When a Pokémon is immobilized, they are prevented from attacking or being attacked (except by the second turn of Sky Drop).

Additionally, the Sky Drop volatile status applies the Fly volatile status and sets a special "is away from field" property. Some effects, such as a Pokémon's Eject Button, cannot activate in this state.

To avoid the nasty glitch from the official games, the core battle engine allows volatile statuses to be `linked` together. When a volatile status ends, all linked effects are also immediately removed. This special handling greatly simplifies the error handling: if the move fails or the user leaves the Sky Drop state for whatever reason, the target's state is cleaned up as well.

To put it all together, Sky Drop applies the following volatile statuses, all linked together:

1. Sky Drop adds the "Immobilizing Move" to the user.
2. The "Immobilizing Move" volatile status adds the "Two Turn Move" and "Sky Drop" volatile statuses to the user.
3. The "Sky Drop" volatile status adds the "Fly" volatile status to the user.
4. The "Immobilizing Move" volatile status adds the "Sky Drop" volatile status to the target.
5. The "Sky Drop" volatile status adds the "Fly" volatile status to the target.

After the move is used, the "Immobilizing Move" is removed from the user, which cleans up all the other linked volatile statuses.

All this to say, Sky Drop is the most complex move in terms of the sheer number of volatile statuses and event callbacks!

### #1 - Substitute

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

```json
{
  "hit_effect": {
    "volatile_status": "substitute"
  },
  "z_move": {
    "effect": "zpowerclearnegativeboosts"
  },
  "effect": {
    "callbacks": {
      "on_try_hit": [
        "require !func_call(has_volatile: $source substitute)",
        "require $source.hp > ($source.max_hp / 4)",
        "require $source.max_hp != 1"
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
        "require $target != $source else return",
        "require !func_call(move_has_flag: $move bypasssubstitute) else return",
        "require !$move.effect_state.infiltrates else return",
        "save_move_hit_data_flag_against_target: $move $target hitsubstitute",
        "# Calculate and apply damage.",
        "$damage = func_call(calculate_damage: $target)",
        "require !$damage.is_boolean",
        "if $damage > $effect_state.hp:",
        ["$damage = $effect_state.hp"],
        "$effect_state.hp = $effect_state.hp - $damage",
        "$move.total_damage = $move.total_damage + $damage",
        "# Break the substitute when HP falls to 0.",
        "if $effect_state.hp == 0:",
        ["if $move.ohko:", ["log_ohko: $target"], "remove_volatile: $target $this.id"],
        "else:",
        ["log_activate: with_target damage"],
        "# Some move effects still apply.",
        "apply_recoil_damage: $damage",
        "apply_drain: $source $target $damage",
        "run_event_on_move: AfterSubstituteDamage",
        "run_event: AfterSubstituteDamage use_source_effect",
        "return 0"
      ],
      "on_try_boost": [
        "require $target != $source and [intimidate, supersweetsyrup] has $effect.id else return",
        "log_fail_unboost: from_effect",
        "return func_call(boost_table)"
      ],
      "on_end": ["log_end"]
    }
  }
}
```

</details>

Substitute trades 25% of the Pokémon's maximum HP for a Substitute doll. The Substitute has its own HP value (equal to the HP lost to create it) and takes damage whenever its creator is targeted. When a move hits into a Substitute, all of its other effects are discarded.

Substitute completely overwrites the move execution flow. It has its own special event, `TryPrimaryHit`, that rewrites how a move is handled. Recoil, HP draining, and some other additional effects occur within the event callback, but overall the move applies zero damage to the actual target.

The core battle engine specially handles when this event returns zero so that it can mark that the move hit into a Substitute for the target. Some other parts of the move execute if a Substitute was hit (e.g., user switches) while others do not (any other secondary effects against the target).

Substitute gains some additional points on the complexity scale given that it was introduced in very first generation. Much of the core battle engine remains hard-coded around the fact that a Substitute could exist, and many other effects account for it as well (checking the "is behind Substitute" state).

## Abilities

We will look at the top 21 most complex abilities.

### #21 - Pickup

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_residual": [
        "require !$target.item else return",
        "$targets = []",
        "foreach $mon in func_call(all_active_mons):",
        [
          "if $mon.last_item.is_defined and $mon.item_used_this_turn and func_call(is_adjacent: $mon $target):",
          ["$targets = func_call(append: $targets $mon)"]
        ],
        "require !$targets.is_empty else return",
        "$random_target = func_call(sample: $targets)",
        "$item = $random_target.last_item",
        "$random_target.last_item = undefined",
        "set_item: $target $item"
      ]
    }
  }
}
```

</details>

The in-battle effect for Pickup was actually introduced in Generation 5. At the end of a turn in which another Pokémon used an item, a Pokémon with Pickup collects that item.

This ability is somewhat complex because it requires the battle engine to track last items (not just for this ability) and whether an item was used in the same turn (exclusive to this ability).

### #20 - Pastel Veil

_Introduced: Generation 8_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_start": ["run_event_on_mon_ability: Update"],
      "on_update": [
        "$activated = false",
        "foreach $ally in func_call(allies_and_self: $mon):",
        [
          "if [psn, tox] has $ally.status:",
          [
            "if !$activated:",
            ["$activated = true", "log_activate: with_target"],
            "cure_status: $ally"
          ]
        ]
      ],
      "on_ally_switch_in": [
        "if [psn, tox] has $mon.status:",
        [
          "log_activate: with_target with_source use_effect_state_target_as_source",
          "cure_status: $mon"
        ]
      ],
      "on_ally_set_status": [
        "if [psn, tox] has $status:",
        [
          "if $effect.is_move and !$effect.is_move_secondary:",
          [
            "if $target == $effect_state.target:",
            ["log_immune: $target from_effect"],
            "else:",
            ["log_block: with_target with_source use_effect_state_target_as_source"]
          ],
          "return stopfail"
        ]
      ]
    }
  }
}
```

</details>

Pastel Veil prevents a Pokémon and its allies from being poisoned. Its complexity mostly arises from the multitude of potential log messages:

- If the ability holder is prevented from being poisoned, the ability is reported as an immunity.
- If an ally is prevented from being poisoned, the action is reported as blocked by the holder's ability.
- If poison is cured for either the holder or an ally, the ability simply activates.

### #19 - Opportunist

_Introduced: Generation 9_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_foe_after_boost": [
        "require $effect.id != $this.id and $effect.id != mirrorherb else return",
        "if !$effect_state.boosts:",
        ["$effect_state.boosts = func_call(boost_table)"],
        "foreach $boost in func_call(boostable_stats):",
        [
          "$value = func_call(get_boost: $boosts $boost)",
          "if $value > 0:",
          [
            "$current = func_call(get_boost: $effect_state.boosts $boost)",
            "$effect_state.boosts = func_call(set_boost: $effect_state.boosts $boost expr($current + $value))"
          ]
        ]
      ],
      "on_after_action": [
        "$boosts = func_call(effect_state_remove_key: $effect_state boosts)",
        "require $boosts.is_defined else return",
        "boost: $mon $boosts"
      ]
    }
  }
}
```

</details>

Opportunist collects all positive stat boosts an opposing Pokémon receives. At the end of an independent action (such as using a move), the Pokémon receives the same stat boosts.

The complexity of this ability mostly stems from its uniqueness. It collects stat boosts on its persistent effect state and clears them at the end of an action. The `AfterAction` event is unique to this effect!

### #18 - Protosynthesis and Quark Drive

_Introduced: Generation 9_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_start": ["run_event_on_mon_ability: WeatherChange"],
      "on_weather_change": [
        "$volatile_state = func_call(volatile_status_effect_state: $target $this.id)",
        "$weather = $field.effective_weather",
        "if $weather.is_defined and $weather.is_sunny:",
        ["add_volatile: $target $this.id"],
        "else if $volatile_state.is_defined and !$volatile_state.from_booster:",
        ["remove_volatile: $target $this.id"]
      ],
      "on_end": [
        "$volatile_state = func_call(volatile_status_effect_state: $target $this.id)",
        "require $volatile_state.is_defined else return",
        "$volatile_state.force_ended = true",
        "remove_volatile: $target $this.id"
      ]
    }
  },
  "condition": {
    "no_copy": true,
    "callbacks": {
      "on_start": [
        "if $source_effect.id == boosterenergy:",
        ["$effect_state.from_booster = true", "log_activate: with_target with_source_effect"],
        "else:",
        ["log_activate: with_target"],
        "$effect_state.best_stat = func_call(best_stat: $target unmodified)",
        "log_start: str('stat:{}', $effect_state.best_stat)"
      ],
      "on_modify_atk": ["if $effect_state.best_stat == atk:", ["return $atk * 13/10"]],
      "on_modify_def": ["if $effect_state.best_stat == def:", ["return $def * 13/10"]],
      "on_modify_spa": ["if $effect_state.best_stat == spa:", ["return $spa * 13/10"]],
      "on_modify_spd": ["if $effect_state.best_stat == spd:", ["return $spd * 13/10"]],
      "on_modify_spe": ["if $effect_state.best_stat == spe:", ["return $spe * 3/2"]],
      "on_end": ["if $effect_state.force_ended:", ["log_end: silent"], "else:", ["log_end"]]
    }
  }
}
```

</details>

Protosynthesis boosts the ability holder's best stat in sunny weather. Quark Drive does the same in Electric Terrain.

However, the ability's volatile status can _also_ be activated with the Booster Energy item. When activated by the item, the effect is not tied to the weather or terrain. This difference requires some activation tracking in the fxlang code.

### #17 - Magician

_Introduced: Generation 6_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_after_move_secondary_effects_user": [
        "require !$user.needs_switch else return",
        "require !$move.damaged_targets.is_empty else return",
        "require !$user.item else return",
        "require !func_call(has_volatile: $user gem) else return",
        "require $move.id != fling else return",
        "require $move.category != status else return",
        "foreach $mon in func_call(speed_sort_mons: $move.damaged_targets):",
        [
          "if $mon == $user:",
          ["continue"],
          "$item = undefined",
          "$item = func_call(take_item: $mon dry_run)",
          "if !$item or !func_call(set_item: $user $item dry_run):",
          ["continue"],
          "$item = func_call(take_item: $mon)",
          "if !$item:",
          ["continue"],
          "set_item: $user $item",
          "break"
        ]
      ]
    }
  }
}
```

</details>

Magician steals a target's item as a secondary effect of the ability holder's move. It has several preconditions before looping through each damaged target and attempting to steal the item.

The item-stealing code is always complex because it must be checked before being executed: we must know that we can both take the target's item and give it to the user. If we cannot give the item to the user, we should not take it from the target in the first place. This is why effects always dry run through both taking and setting the item.

### #16 - Forewarn

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_start": [
        "$foes = []",
        "$moves = []",
        "$max_base_power = 1",
        "foreach $foe in func_call(all_foes: $target):",
        [
          "foreach $move_slot in $foe.move_slots:",
          [
            "$move = func_call(get_move: $move_slot.id)",
            "if !$move:",
            ["continue"],
            "$base_power = $move.base_power",
            "if $move.ohko:",
            ["$base_power = 150"],
            "if [counter, metalburst, mirrorcoat] has $move.id:",
            ["$base_power = 120"],
            "if $base_power == 0 and $move.category != status:",
            ["$base_power = 80"],
            "if $base_power > $max_base_power:",
            ["$foes = [$foe]", "$moves = [$move]", "$max_base_power = $base_power"],
            "else if $base_power == $max_base_power:",
            ["$foes = func_call(append: $foes $foe)", "$moves = func_call(append: $moves $move)"]
          ]
        ],
        "require !$moves.is_empty else return",
        "$i = func_call(random: $moves.length)",
        "$foe = func_call(index: $foes $i)",
        "$move = func_call(index: $moves $i)",
        "log_activate: with_target str('move:{}', $move.name) str('of:{}', $foe.position_details)"
      ]
    }
  }
}
```

</details>

Forewarn reveals an opponent's strongest move by base power. Of course, not all damaging moves have a static base power, so Forewarn assigns static values for such moves (for example, one-hit KO moves are treated as having 150 base power).

This ability is complex mostly because there is nothing else like it. The ability must track ties (resolved randomly) and new maximums (which clear out the existing list).

### #15 - Trace

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_start": ["run_event_on_mon_ability: Update"],
      "on_update": [
        "$targets = []",
        "foreach $target in func_call(adjacent_foes: $mon):",
        [
          "if !func_call(ability_has_flag: $target.ability permanent) and !func_call(ability_has_flag: $target.ability notrace):",
          ["$targets = func_call(append: $targets $target)"]
        ],
        "require !$targets.is_empty else return",
        "$target = func_call(sample: $targets)",
        "set_ability: $mon $target.ability"
      ]
    }
  }
}
```

</details>

Trace changes the Pokémon's ability into a random opponent's ability whenever possible. Trace can activate as soon as a Pokémon enters the battle. If Trace is unable to find an ability to copy, it will continue to look for one throughout the battle.

Trace is an interesting ability in terms of how frequently it attempts to activate. Additionally, the Ability Shield item blocks Trace from activating. When a Pokémon loses its Ability Shield (such as from being hit by Knock Off), Trace immediately activates.

### #14 - Forme-Changing Abilities

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

Forecast:

```json
{
  "effect": {
    "callbacks": {
      "on_start": ["run_event_on_mon_ability: WeatherChange"],
      "on_weather_change": [
        "require func_call(base_species: $target) == castform else return",
        "$weather = func_call(effective_weather: $target use_target_as_origin)",
        "if !$weather:",
        ["$forme = castform"],
        "else if $weather.is_raining:",
        ["$forme = castformrainy"],
        "else if $weather.is_sunny:",
        ["$forme = castformsunny"],
        "else if $weather.is_snowing:",
        ["$forme = castformsnowy"],
        "else:",
        ["$forme = castform"],
        "if $target.species != $forme:",
        ["forme_change: $target $forme"]
      ],
      "on_end": [
        "require func_call(base_species: $target) == castform else return",
        "forme_change: $target castform"
      ]
    }
  }
}
```

Zen Mode:

```json
{
  "effect": {
    "callbacks": {
      "on_residual": [
        "require func_call(base_species: $target) == darmanitan else return",
        "if $target.hp <= $target.max_hp / 2:",
        ["add_volatile: $target $this.id"],
        "else:",
        ["remove_volatile: $target $this.id"]
      ],
      "on_end": ["remove_volatile: $target $this.id"]
    }
  },
  "condition": {
    "callbacks": {
      "on_start": [
        "if $target.species == darmanitan:",
        ["forme_change: $target darmanitanzen"],
        "else if $target.species == darmanitangalar:",
        ["forme_change: $target darmanitangalarzen"]
      ],
      "on_end": [
        "if $target.species == darmanitanzen:",
        ["forme_change: $target darmanitan"],
        "else if $target.species == darmanitangalarzen:",
        ["forme_change: $target darmanitangalar"]
      ]
    }
  }
}
```

Stance Change:

```json
{
  "effect": {
    "callbacks": {
      "on_use_move": [
        "require func_call(base_species: $user) == aegislash else return",
        "require $move.category != status or $move.id == kingsshield else return",
        "$species = aegislashblade",
        "if $move.id == kingsshield:",
        ["$species = aegislash"],
        "if $user.species != $species:",
        ["forme_change: $user $species"]
      ]
    }
  }
}
```

Shields Down:

```json
{
  "effect": {
    "callbacks": {
      "on_start": ["run_event_on_mon_ability: Residual"],
      "on_residual": [
        "$base_species = func_call(base_species: $target)",
        "require $base_species == minior and $target.hp != 0 else return",
        "if $target.hp > $target.max_hp / 2:",
        ["forme_change: $target miniormeteor"],
        "else:",
        [
          "# Original base species preserves the Minior color.",
          "forme_change: $target $target.original_base_species"
        ]
      ],
      "on_set_status": [
        "require $target.species == miniormeteor else return",
        "if $effect.is_move and !$effect.is_move_secondary:",
        ["log_immune: from_effect"],
        "return stopfail"
      ],
      "on_add_volatile": [
        "require $target.species == miniormeteor else return",
        "if $volatile.id == yawn:",
        [
          "if $effect.is_move and !$effect.is_move_secondary:",
          ["log_immune: from_effect"],
          "return stopfail"
        ]
      ]
    }
  }
}
```

Gulp Missile:

```json
{
  "effect": {
    "callbacks": {
      "on_before_charge_move": ["require $move.id == dive else return", "activate_ability"],
      "on_source_try_primary_hit": [
        "require $move.id == surf else return",
        "activate_ability: $source"
      ],
      "on_activate": [
        "require $target.species == cramorant else return",
        "$forme = cramorantgulping",
        "if $target.hp <= $target.max_hp / 2:",
        ["$forme = cramorantgorging"],
        "forme_change: $target $forme"
      ],
      "on_damaging_hit": [
        "require $source.hp != 0 else return",
        "require $source.active else return",
        "require !$target.is_semi_invulnerable else return",
        "require $target.species != cramorant else return",
        "$base_species = func_call(base_species: $target)",
        "require $base_species == cramorant else return",
        "damage: $source expr($source.base_max_hp / 4)",
        "if $target.species == cramorantgulping:",
        ["boost: $source 'def:-1'"],
        "else if $target.species == cramorantgorging:",
        ["set_status: $source par"],
        "forme_change: $target cramorant"
      ]
    }
  }
}
```

</details>

Many abilities change a specific Pokémon's forme, either permanently (preserved across switches) or temporarily. Formes can change under a variety of scenarios:

- Weather (Forecast).
- HP (Zen Mode, Shields Down, Schooling, Power Construct).
- Using a move (Stance Change, Gulp Change).
- Fainting a Pokémon (Battle Bond).
- Every turn (Hunger Switch).
- Switching out (Zero to Hero).

It may seem oversimplified to group all of these different abilities together, but they are conceptually similar. When some condition activates, the current and intended species of a Pokémon must be compared, triggering a forme change if necessary.

Forme-changing abilities do not activate when gained via Transform. Most abilities have both a base species check (e.g., Zen Mode, for instance, cannot activate if the holder's base species is not Darmanitan) and a transform check (implemented as a "NoTransform" flag that suppresses the ability if the holder is transformed).

### #13 - Multitype and RKS System

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_types": {
        "order": 1,
        "program": [
          "# Each Arceus forme has its own species data, so the type of each forme is actually set directly by the forme.",
          "# Multitype technically alters the Mon's type directly, so we implement dynamic type changing logic here as well.",
          "if !$mon.item:",
          ["return $types"],
          "$type = normal",
          "$data = func_call(special_item_data: $mon.item)",
          "if $data.is_defined and $data.judgment.is_defined:",
          ["$type = $data.judgment.type.to_string"],
          "return [$type]"
        ]
      },
      "on_update": [
        "# Ensure Arceus is the correct forme.",
        "# Note that when team validation is used, Arceus is forced into the correct forme.",
        "require func_call(base_species: $mon) == arceus and $mon.item.is_defined else return",
        "$type = normal",
        "$data = func_call(special_item_data: $mon.item)",
        "if $data.is_defined and $data.judgment.is_defined:",
        ["$type = $data.judgment.type.to_string"],
        "$species = func_call(get_species: str('arceus-{}', $type))",
        "require $species.is_defined else return",
        "if $mon.species != $species.id:",
        ["forme_change: $mon $species.id permanent"]
      ],
      "on_take_item": [
        "# Item cannot be taken if Multitype is using it.",
        "$data = func_call(special_item_data: $item.id)",
        "return !$data or !$data.judgment"
      ],
      "on_set_item": [
        "# Item cannot be given if Multitype would use it.",
        "$data = func_call(special_item_data: $item.id)",
        "return !$data or !$data.judgment"
      ],
      "on_set_types": ["return false"]
    }
  }
}
```

</details>

Multitype changes Arceus' forme based on the type of plate it is holding. RKS System is the equivalent for Silvally with a memory item.

These abilities are implemented with additional checks just to ensure total correctness and consistency.

First, Arceus and Silvally technically have one forme for each type. Thus, the ability does not _really_ change the type of Arceus and Silvally, it just forces them into the correct forme. The `Update` event covers this: if for some reason Arceus/Silvally enters the battle in the wrong forme, it changes forme as soon as possible.

However, these abilities, by description, do technically change the Pokémon's type. The type changing logic is implemented separately. Although it does not really do anything for Arceus and Silvally, if another Pokémon somehow got one of these abilities, their type would change.

Additionally, a Pokémon cannot lose its item if Multitype or RKS System is using it. This prevents the Pokémon from transforming into a different forme mid-battle.

The implementation of these abilities may be overkill, but they are the most literal implementations for both the type-changing and forme-changing properties of these abilities!

### #12 - Ripen

_Introduced: Generation 8_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_try_heal": [
        "if $effect.id == berryjuice or $effect.id == leftovers or ($effect.is_item and func_call(item_has_flag: $effect.id berry)):",
        ["log_activate: with_target", "return $damage * 2"]
      ],
      "on_try_boost": [
        "if $effect.is_item and func_call(item_has_flag: $effect.id berry):",
        [
          "foreach $stat in func_call(boostable_stats):",
          [
            "$boosts = func_call(set_boost: $boosts $stat expr(func_call(get_boost: $boosts $stat) * 2))"
          ],
          "return $boosts"
        ]
      ],
      "on_try_eat_item": {
        "priority": -1,
        "program": ["log_activate: with_target"]
      },
      "on_eat_item": [
        "if func_call(item_has_flag: $item.id damagereducingberry):",
        ["$effect_state.berry_weaken = true"]
      ],
      "on_source_modify_damage": {
        "priority": -1,
        "program": [
          "if $effect_state.berry_weaken:",
          ["$effect_state.berry_weaken = false", "return $damage / 2"]
        ]
      },
      "on_damage": [
        "if $effect.is_item and func_call(item_has_flag: $effect.id berry):",
        ["return $damage * 2"]
      ],
      "on_restore_pp": [
        "if $effect.is_item and func_call(item_has_flag: $effect.id berry):",
        ["return $pp * 2"]
      ]
    }
  }
}
```

</details>

Ripen doubles the effects of berries eaten in battle. Of course, "doubling" depends on the context:

- Healing effects heal twice as much HP.
- Boosting effects boost twice as much.
- Damage-reducing berries reduce twice as much damage.
- PP-restoring effects restore twice as much PP.
- Damage-dealing berries deal twice as much damage.

All of these doubling effects must be implemented on the same ability, which gives this ability one of the most wide-reaching effects.

### #11 - Berserk and Anger Shell

_Introduced: Generation 7_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "delegates": ["condition:hpactivatedabilitybase"],
    "callbacks": {
      "on_activate": ["boost: $target 'spa:1'"]
    }
  }
}
```

HP Activated Ability Base:

```json
{
  "condition": {
    "callbacks": {
      "on_after_damage": [
        "# Delay eating healing berries if we expect Berserk to activate for a single-hit move.",
        "if $effect.is_move and !$effect.multihit and !$effect.ignore_all_secondary_effects:",
        ["$effect_state.ability_may_activate = true"]
      ],
      "on_after_move_secondary_effects_damage": [
        "require $damage != 0 else return",
        "$effect_state.ability_may_activate = false",
        "require $original_hp.is_defined else return",
        "require $target.hp != 0 else return",
        "$half = $target.max_hp / 2",
        "if $original_hp > $half and $target.hp <= $half:",
        ["activate_ability"]
      ],
      "on_after_move": [
        "# If secondary effects were suppressed, we still need to reset state.",
        "$effect_state.ability_may_activate = false"
      ],
      "on_try_eat_item": [
        "# Healing items are consumed because HP dropped below a threshold.",
        "# If we received damage as part of a single-hit move, don't consume the berry until Berserk has activated.",
        "$healing_items = [aguavberry, enigmaberry, figyberry, iapapaberry, magoberry, sitrusberry, wikiberry, oranberry, berryjuice]",
        "require !($healing_items has $item.id) or !$effect_state.ability_may_activate else return stopfail"
      ]
    }
  }
}
```

</details>

Berserk boosts the ability holder's Special Attack each time its HP drops below half due to a damaging move. Anger Shell has a similar effect but with a wider array of boosts, increasing attack stats and speed while dropping defenses.

Due to the activation conditions being the same, the triggering logic is shared in an ability base. The effect activates as a secondary effect of the move (each move tracks a target's original HP, specifically for this type of check).

One catch is that a healing berry should not be eaten if the move hits only once and the ability would activate. This rule benefits the user: stat boosts are granted before HP is restored above 50%. This edge case is implemented by preventing a berry from being eaten until after a move finishes executing.

The activation timing of these abilities is very delicate and requires some juggling with berry activation timing.

### #10 - Illusion

_Introduced: Generation 5_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_switching_in": [
        "foreach $target in func_call(reverse: $mon.player.team_by_effective_position):",
        [
          "if $target == $mon or $target.exited:",
          ["continue"],
          "set_illusion: $mon $target",
          "break"
        ]
      ],
      "on_damaging_hit": [
        "require $target.illusion.is_defined else return",
        "end_ability: $target silent"
      ],
      "on_end": [
        "require $target.illusion.is_defined else return",
        "end_illusion: $target",
        "log_end"
      ],
      "on_before_terastallization": [
        "require $mon.illusion.is_defined else return",
        "$base_species = func_call(lookup_base_species: $mon.illusion)",
        "if [ogerpon, terapagos] has $base_species:",
        ["end_ability: $mon silent"]
      ]
    }
  }
}
```

</details>

Illusion changes the Pokémon's physical appearance to match that of the Pokémon in the player's last party position. The illusion breaks when the Pokémon is hit by a damaging move.

Illusion is not as complex as it sounds. The battle log already presents Pokémon in the battle log in specific ways (full details in switch logs, partial details in any other log). The battle engine simply tracks a volatile property for the Pokémon's physical appearance, which can be added and removed by the ability.

The volatile physical appearance and the tracking for which Pokémon is the last in the player's party (which changes dynamically based on switches) are completely unique to this ability.

### #9 - Disguise and Ice Face

_Introduced: Generation 7_

<details>
<summary><i>Code</i></summary>

Disguise:

```json
{
  "effect": {
    "callbacks": {
      "on_damage": {
        "priority": 1,
        "program": [
          "require $effect.is_move and [mimikyu, mimikyutotemdisguised] has $target.species else return",
          "activate_ability",
          "return 0"
        ]
      },
      "on_activate": [
        "$species = mimikyubusted",
        "if $target.species == mimikyutotemdisguised:",
        ["$species = mimikyutotembusted"],
        "forme_change: $target $species permanent",
        "damage: $target expr($target.base_max_hp / 8)"
      ],
      "on_critical_hit": [
        "require [mimikyu, mimikyutotemdisguised] has $target.species else return",
        "require !func_call(move_hit_data_has_flag_against_target: $move $target hitsubstitute) else return",
        "require !func_call(check_move_immunity: $target $move) else return",
        "return false"
      ],
      "on_effectiveness": [
        "require $effect.category != status else return",
        "if func_call(run_event_on_mon_ability: CriticalHit use_source_effect no_forward) == false:",
        ["return 0"]
      ]
    }
  }
}
```

Ice Face:

```json
{
  "effect": {
    "callbacks": {
      "on_damage": {
        "priority": 1,
        "program": [
          "require $effect.is_move else return",
          "require $effect.category == physical else return",
          "require $target.species == eiscue else return",
          "forme_change: $target eiscuenoice permanent",
          "return 0"
        ]
      },
      "on_critical_hit": [
        "require $target.species == eiscue else return",
        "require $move.category == physical else return",
        "require !func_call(move_hit_data_has_flag_against_target: $move $target hitsubstitute) else return",
        "require !func_call(check_move_immunity: $target $move) else return",
        "return false"
      ],
      "on_effectiveness": [
        "if func_call(run_event_on_mon_ability: CriticalHit use_source_effect no_forward) == false:",
        ["return 0"]
      ],
      "on_weather_change": [
        "require $target.species == eiscuenoice else return",
        "$weather = func_call(effective_weather: $target use_target_as_origin)",
        "if $weather.is_defined and $weather.is_snowing:",
        ["forme_change: $target eiscue permanent"]
      ],
      "on_start": ["run_event_on_mon_ability: WeatherChange"]
    }
  }
}
```

</details>

Disguise and Ice Face were purposefully left out of the "Forme-Changing Abilities" section earlier. Both abilities completely consume an initial damaging hit when the Pokémon is in a particular forme:

- Disguise prevents a damaging hit when Mimikyu is in its Disguised forme. It transforms into its Busted forme permanently afterwards.
- Ice Face prevents a physical damaging hit when Eiscue is in its Ice Face forme. It transforms into its Noice forme afterwards. Ice Face can be restored if the weather changes to Hail or Snow.

These abilities are complex because they negate damage, critical hits, and type effectiveness for the initial damaging hit. They add an extra layer of complexity on top of other forme-changing abilities.

Additionally, Disguise must account for Mimikyu's Totem forme.

### #8 - Sheer Force

_Introduced: Generation 5_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_use_move": [
        "require !$move.secondary_effects.is_empty else return",
        "$move.secondary_effects = []",
        "$move.user_effect = undefined",
        "$move.ignore_all_secondary_effects = true"
      ],
      "on_source_base_power": [
        "if $move.ignore_all_secondary_effects:",
        ["return $base_power * 13/10"]
      ]
    }
  }
}
```

</details>

Sheer Force boosts the base power of moves by 30% at the cost of removing all secondary effects. The core battle engine must account for skipping secondary effects, since they are baked into the move execution flow via static fields (e.g., stat boosts, chances to apply a status) and custom events.

Sheer Force requires us to implement anything that counts as a secondary effect correctly. For example, it is possible for a damaging move to also define a primary hit effect to apply some status or boost, but this would be incorrect. Primary hit effects are not skipped while secondary effects are. Additionally, several custom effects are considered secondary effects, such as thawing a target, ability effects such as Color Change or Berserk, item effects such as Life Orb or Shell Bell, and many others.

The large amount of interactions with Sheer Force gives it a decent ranking on the complexity scale.

### #7 - Mold Breaker

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "delegates": ["condition:ignoreabilitymovebase"],
    "callbacks": {
      "on_start": ["log_ability"]
    }
  },
  "condition": {
    "duration": 1,
    "callbacks": {
      "suppress_mon_ability": [
        "if func_call(ability_has_flag: $mon.ability breakable):",
        ["return true"]
      ]
    }
  }
}
```

Ignore Ability Move Base:

```json
{
  "condition": {
    "duration": 1,
    "callbacks": {
      "on_before_move": [
        "$category = func_call(value_from_local_data: category)",
        "require !$category or $move.category == $category else return",
        "add_pseudo_weather: moldbreaker"
      ],
      "on_move_aborted": ["run_event_on_move: AfterMove"],
      "on_after_move": ["remove_pseudo_weather: moldbreaker"]
    }
  }
}
```

</details>

Mold Breaker suppresses the effects of 'breakable' abilities while the ability holder uses a move. For example, a suppressed Wonder Guard allows an attacker to hit the target without a super-effective move.

Mold Breaker is implemented as a pseudo-weather that suppresses abilities. The pseudo-weather is added before the move and removed afterwards. This process of adding a pseudo-weather during a move is encapsulated in a move base, so moves that ignore abilities (e.g., Sunsteel Strike, Moongeist Beam) can reuse the same effect.

Mold Breaker is complex because it is implemented in such a unique way. Additionally, ability suppression as a whole is already complex, so handling it dynamically during a move's execution adds to a battle's dynamism.

### #6 - Dancer

_Introduced: Generation 7_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_start": ["add_pseudo_weather: $this.id"],
      "on_end": [
        "# If no other Mons have this ability on the field, remove the pseudo-weather.",
        "foreach $other in func_call(all_active_mons):",
        ["require $other == $target or !func_call(has_ability: $other $this.id) else return"],
        "remove_pseudo_weather: $this.id"
      ]
    }
  },
  "condition": {
    "callbacks": {
      "on_after_move": [
        "require func_call(move_has_flag: $move dance) else return",
        "require $success else return",
        "require !$move.external else return",
        "$dancers = []",
        "foreach $mon in func_call(reverse: func_call(all_active_mons_in_speed_order_and_ability_effect_order)):",
        [
          "if $mon == $user:",
          ["continue"],
          "if func_call(has_ability: $mon $this.id) and !$mon.is_semi_invulnerable:",
          ["$dancers = func_call(append: $dancers $mon)"]
        ],
        "foreach $dancer in $dancers:",
        [
          "faint_messages",
          "require !$battle.ending else return",
          "require !$dancer.fainted else return",
          "log_activate: with_target $dancer",
          "if func_call(is_ally: $dancer $user):",
          ["$move_target = $target"],
          "else:",
          ["$move_target = $user"],
          "$dancer_move = func_call(new_active_move: $move.id $dancer)",
          "use_active_move: $dancer $dancer_move $move_target use_effect_as_source_effect preventable"
        ]
      ]
    }
  }
}
```

</details>

When another Pokémon uses a dancing move, all Pokémon with the Dancer ability will immediately use the move as well.

Dancer has one of the most unique implementations:

1. A pseudo-weather on the field tracks Dancer's activation after a dance move.
2. External moves (e.g., not an independent move action) are not copied. This prevents infinite recursion.
3. The ability activates in reverse speed order.
4. Before a move is used, the effect must check that the battle is not ending.
5. The dance move is executed against the proper target (allies attack the same target, foes attack the user).

Dancer has the opportunity to execute the largest number of moves per battle if multiple Pokémon with the ability use a dance move.

### #5 - As One

_Introduced: Generation 8_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_before_start": [
        "$effect_state.activated = false",
        "clear_sub_abilities: $target",
        "if $target.species == calyrexice:",
        ["add_sub_ability: $target unnerve", "add_sub_ability: $target chillingneigh"],
        "else if $target.species == calyrexshadow:",
        ["add_sub_ability: $target unnerve", "add_sub_ability: $target grimneigh"],
        "else:",
        ["return"],
        "$effect_state.activated = true"
      ],
      "on_start": [
        "require !$effect_state.announced and $effect_state.activated else return",
        "log_ability",
        "$effect_state.announced = true"
      ]
    }
  }
}
```

</details>

As One combines the effects of two abilities into one, depending on the Calyrex forme with the ability:

- Calyrex Ice Rider has the Unnerve and Chilling Neigh abilities.
- Calyrex Shadow Rider has the Unnerve and Grim Neigh abilities.

fxlang already supports "delegate" effects, which copies the event callbacks from one effect to another (e.g., Dry Skin copies the Water Absorb effect, adding the residual damage/healing effect based on the weather). However, a delegate effect is a strict copy: activation logs (assuming `$this` is used, which is correct) will show the name of the real effect, not the copied one (e.g., logs do not act like a Pokémon with Dry Skin has Water Absorb).

As One is different: battle logs actually display the two abilities disjointedly. Thus, As One cannot use delegate effects as is, unless we specifically wrote Unnerve, Chilling Neigh, and Grim Neigh to hard-code their names in battle logs.

Additionally, in the official games, As One is actually implemented as two separate abilities with the same name given to the different Calyrex formes. They simply hard-code the combination of the two intended abilities.

Implementing As One with both of these solutions (hard-coding logs, hard-coding the combination), felt cheap. Isn't it much more interesting to actually support ability combination?

To support As One in a reusable, extendable way, the core battle engine introduced the notion of "sub-abilities." An ability can add sub-abilities to itself before starting. When an event runs on an ability, it _also_ runs on the sub-abilities.

With this implementation, As One's fxlang code does exactly what the ability says it does: it grants the Pokémon two abilities at the same time.

While sub-abilities were not required to implement this ability, the large number of considerations for either approach makes this ability one of the most complex and interesting. While most abilities that combine other ability effects are implemented with delegates (e.g., the new Eelevate ability copies the effects of Levitate and Beast Boost), it is interesting to consider more types of abilities that act like As One.

### #5 - Mega Sol

_Introduced: Generation 9_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_start_using_move": ["add_volatile: $this.id"],
      "on_stop_using_move": ["remove_volatile: $this.id"]
    }
  },
  "condition": {
    "duration": 1,
    "callbacks": {
      "on_override_weather": ["return harshsunlight"],
      "on_before_charge_move": [
        "# Solar Beam will see that the weather is sunny, though it may not actually be.",
        "# Show the ability to make the effect apparent.",
        "if $move.id == solarbeam and (!$field.weather or !$field.weather.is_sunny):",
        ["log_ability"]
      ]
    }
  }
}
```

</details>

When a Pokémon with Mega Sol uses a move, the move behaves as if the weather is sunny, regardless of the actual weather or weather-suppressing effects.

This ability is the first of its kind: while weather could be suppressed on a per-Pokémon basis since Generation 8 (via the Utility Umbrella), it could never be _overridden_ entirely! Implementing this ability required us to introduce weather overrides in the core battle engine and fxlang.

Additionally, during a move's execution, one Pokémon could override the effective weather for _another_ Pokémon. For example, moves ignore the Special Defense boost granted to Rock-type Pokémon in a sandstorm. This means that the `ModifySpd` event callback for the Sandstorm weather should not run while the weather is overridden by Mega Sol.

To implement this overriding, the core battle engine tracks the "origin" Pokémon of an event, which represents the primary reason an event was triggered. When a target's Special Defense is being modified, the origin of this effect is the move user. The core battle engine calculates a Pokémon's effective weather relative to the origin of the event: if the origin Pokémon has a weather override, that weather is preferred.

This implementation may feel like overkill, but it is actually the most correct. We do not need to hard-code the consideration for weather overrides in all weather-impacted event callbacks; the core battle engine handles this complexity for us.

However, there are cases where a weather override should not be respected. For example, if a Pokémon with Mega Sol uses the move Rainy Dance, Mega Sol could force Castform into its Sunny forme by the rules of weather overrides. However, this interaction would make no sense. As a result, weather-responding effects such as the Forecast ability must be modified to not respect weather overrides.

All this to say, Mega Sol's implementation may look simple on the surface, but the core battle engine does a lot of heavy lifting to make weather overrides as transparent to all other battle effects as possible.

### Honorable Mentions

Here are some abilities, in no particular order, that did not make the list but are interesting nonetheless:

1. Beast Boost (Gen 7) - Boosts the Pokémon's best stat by the number of Pokémon it caused to faint.
2. Comatose (Gen 7) - The Pokémon is treated as if it is always asleep.
3. Contrary (Gen 5) - Flips stat boosts and drops.
4. Costar (Gen 9) - Copies an adjacent ally's stat boosts and drops.
5. Cud Chew (Gen 9) - Consumes an eaten berry a second time two turns later.
6. Delta Stream (Gen 6) - Starts the Strong Winds weather, which protects Flying-type Pokémon from super-effective damage. The weather stays active while any Pokémon with Delta Stream is active, or until another strong weather is set on the field.
7. Flash Fire (Gen 3) - Grants immunity to Fire-type attacks, activating an effect that boosts both attack stats when hit.
8. Flower Veil (Gen 6) - Protects Grass-type allies from stat drops and non-volatile statuses.
9. Magic Bounce (Gen 5) - Reflects certain moves back at the attacker (same as Magic Coat).
10. Moody (Gen 5) - Randomly boosts one stat by two stages and drops another stat by one stage at the end of each turn.
11. Infiltrator (Gen 5) - Bypasses screen-based side conditions, as well as Substitutes.
12. Parental Bond (Gen 6) - Turns every attacking move into a two-hit move. The second hit deals quarter damage.
13. Prankster (Gen 5) - Boosts the priority of status moves. Dark types are immune to Prankster-boosted moves.
14. Slow Start (Gen 4) - Halves attack and speed for five turns.
15. Stench (Gen 3) - Adds a 10% chance to cause the target to flinch for all attacking moves (unless it already has a chance to flinch).
16. Wandering Spirit (Gen 8) - Swaps abilities with an attack on a damaging hit.

### #3 - Neutralizing Gas

_Introduced: Generation 8_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_before_switch_in": [
        "log_ability",
        "# Use a pseudo-weather for suppression two reasons:",
        "# 1. A pseudo-weather affects all Mons on the field with one condition.",
        "# 2. Due to the implementation of how we avoid infinite recursion in event callbacks, abilities cannot suppress other abilities.",
        "add_pseudo_weather: $this.id"
      ],
      "on_end": [
        "# If no other Mons have this ability on the field, remove the pseudo-weather.",
        "foreach $other in func_call(all_active_mons):",
        ["require $other == $target or !func_call(has_ability: $other $this.id) else return"],
        "log_end",
        "remove_pseudo_weather: $this.id"
      ]
    }
  },
  "condition": {
    "callbacks": {
      "suppress_mon_ability": [
        "# Do not suppress our own ability, so that we can trigger this condition to end.",
        "if $effect_state.started and $mon.ability != $this.id:",
        ["return true"]
      ],
      "on_field_start": [
        "# If a Mon's ability is suppressed, end it.",
        "# Note that `end_ability` has no effect if the ability was already ended elsewhere, or never started.",
        "foreach $mon in func_call(all_active_mons_in_speed_order):",
        ["if $mon.can_suppress_ability:", ["end_ability: $mon silent"]]
      ],
      "on_field_end": [
        "# Restart abilities.",
        "# Note that `start_ability` has no effect if an ability is still suppressed.",
        "foreach $mon in func_call(all_active_mons_in_speed_order):",
        ["start_ability: $mon silent"]
      ]
    }
  }
}
```

</details>

Neutralizing Gas suppresses the abilities of _all_ Pokémon on the field.

It is implemented by adding a pseudo-weather to the field to suppress abilities. A pseudo-weather captures the fact that multiple Pokémon with Neutralizing Gas all trigger the same effect (and the effect does not lift until all of them are off the field). Additionally, it prevents infinite recursion in the battle engine: an ability cannot suppress abilities, because in order to determine if an ability is suppressed, we would need to run a Pokémon's ability event callback (confused yet?).

When a Pokémon's ability is suppressed, the ability's `End` event must run (in order for the ability to trigger any cleanup or remove any effects). Likewise, when Neutralizing Gas is lifted, abilities must be restarted.

As mentioned earlier with ability-suppressing moves, ability suppression in general is very complex. The core battle engine does some heavy lifting for us: it caches whether an ability is suppressed, whether it could be suppressed, and whether it has started. Neutralizing Gas leverages these properties heavily:

- When started, an ability is ended only if the ability can be suppressed. At this stage, the suppression is not active.
- When active (started), each Pokémon's ability is suppressed.
- When ending, an ability is started. Internally, if the ability is already started, nothing happens.

Additionally, Neutralizing Gas cannot suppress itself, or else the event callback to end the effect would not run.

Neutralizing Gas's global effect on the field, as well as the complexities of suppression itself, makes this ability quite tricky to implement.

### #2 - Commander

_Introduced: Generation 9_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_any_switch_in": ["activate_ability: $effect_state.target"],
      "on_start": ["activate_ability: $target"],
      "on_end": ["remove_volatile: $target commanding"],
      "on_activate": [
        "require !$target.needs_switch else return",
        "if func_call(base_species: $target) == tatsugiri:",
        [
          "$commanding_state = func_call(volatile_status_effect_state: $target commanding)",
          "if $commanding_state.is_defined:",
          ["$ally = $commanding_state.source"],
          "else:",
          [
            "foreach $mon in func_call(adjacent_allies):",
            [
              "if !$mon.needs_switch and func_call(base_species: $mon) == dondozo and !func_call(has_volatile: $mon commanded):",
              ["$ally = $mon"]
            ]
          ]
        ],
        "if !$ally:",
        ["remove_volatile: $target commanding", "return"],
        "if !func_call(has_volatile: $target commanding):",
        ["log_activate: with_target", "add_volatile: $ally commanded"]
      ]
    }
  }
}
```

Commanded:

```json
{
  "condition": {
    "no_copy": true,
    "callbacks": {
      "on_start": [
        "require $source.is_defined",
        "add_volatile: $source commanding with_source link",
        "boost: $target use_source_effect use_effect_state_source_as_source 'atk:2' 'spa:2' 'def:2' 'spd:2' 'spe:2'"
      ],
      "on_drag_out": ["return false"],
      "on_trap_mon": ["return true"]
    }
  }
}
```

Commanding:

```json
{
  "condition": {
    "no_copy": true,
    "callbacks": {
      "on_start": ["log_start: with_source", "cancel_action: $target"],
      "on_end": ["log_end"],
      "is_away_from_field": ["return true"],
      "on_before_turn": ["cancel_action: $user"],
      "on_drag_out": ["return false"],
      "on_lock_move": ["return pass"],
      "on_invulnerability": {
        "order": 1,
        "program": ["return false"]
      }
    }
  }
}
```

</details>

Commander causes Tatsugiri to leap inside an ally Dondozo's mouth, granting Dondozo massive stat boosts. While Tatsugiri is inside Dondozo, it is unable to move or be hit by nearly anything.

Commander is implemented as two volatile statuses:

1. The ability starts. Tatsugiri leaps into a non-commanded Dondozo, granting it the "Commanded" volatile status.
2. The "Commanded" volatile status adds the "Commanding" volatile status to the source (Tatsugiri).
3. The "Commanding" volatile status locks the Pokémon into passing (unable to make any moves), grants it total invulnerability, and marks it as "away from the field" (similar to Sky Drop).

Commander is quite an interesting ability. It does not require the same heavy lifting from the battle engine as other complex effects, but it is very complex in its own right in terms of its fxlang code and impact on the battle (one Pokémon is practically removed from the field while in the commanding state).

### #1 - Emergency Exit and Wimp Out

_Introduced: Generation 7_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_after_damage": [
        "# Move damage is handled together after secondary effects.",
        "require !$effect.is_move and $effect.id != confusion else return",
        "$original_hp = $target.hp + $damage",
        "activate_ability: $original_hp"
      ],
      "on_after_move_secondary_effects_damage": [
        "require $damage != 0 else return",
        "activate_ability: $original_hp"
      ],
      "on_activate": {
        "metadata": {
          "parameters": ["original_hp"]
        },
        "program": [
          "require $original_hp.is_defined else return",
          "require !$battle.ending else return",
          "require func_call(can_switch: $target.player) else return",
          "require $target.hp != 0 else return",
          "require !$target.force_switch else return",
          "require !$target.needs_switch else return",
          "require !$target.is_away_from_field else return",
          "$half = $target.max_hp / 2",
          "if $original_hp > $half and $target.hp <= $half:",
          [
            "foreach $mon in func_call(all_active_mons):",
            ["set_needs_switch: $mon false"],
            "log_activate: with_target",
            "if $target.player.wild_encounter_type.is_defined:",
            ["escape: $target"],
            "else:",
            ["switch_out: $target"]
          ]
        ]
      }
    }
  }
}
```

</details>

Emergency Exit and Wimp Out have the exact same effect. When a Pokémon's HP drops below half for _any_ reason (not just moves), it switches out immediately.

The activation condition for Emergency Exit may look similar to Berserk. However, Berserk can only activate from move-inflicted damage; Emergency Exit can activate for any damage. On its own, this change is not too tricky, but move-inflicted damage works differently: Emergency Exit only activates after the last hit of a multi-hit move. Thus, the activation logic must account for where the damage is coming from and what the Pokémon's original HP was before the damage.

Additionally, Emergency Exit switches the user out immediately. Typically, mid-turn switches are activated in the normal move execution flow (there is a standard field for switching a user out, such as during U-turn). These switches wait until around the end of the move to perform the switch out. Emergency Exit works differently: the Pokémon can switch out basically whenever. For example, a Pokémon with Emergency Exit can switch in, be damaged by entry hazards, then immediately switch out!

This different timing forces us to account for a Pokémon being inactive at stages of the battle when it normally would not be. The core battle engine and many other functions needed to be much more strict with their active checks.

Finally, Emergency Exit cancels out any pending switch action. For example, a user switch incurred by using U-turn will not execute if Emergency Exit activates.

All in all, Emergency Exit is incredibly tedious in its activation timing. Some nuances of how it works largely depend on how the battle engine itself is implemented. The approach outlined above gets us as close to the mainline game behavior as possible, given the current battle engine organization. Nonetheless, there are still some gaps: Shell Bell should activate before Emergency Exit activates (even if HP rises above 50%), but our current implementation does not do this (and it is unclear how to do this at all).

## Items

We will look at the top 17 most complex items.

In general, items are much simpler than moves or abilities. Still, it is interesting to see how they stack up.

### #17 - Damage-Reducing Berries

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

Occa Berry:

```json
{
  "effect": {
    "delegates": ["condition:damagereducingberryitembase"],
    "local_data": {
      "values": {
        "type": "Fire"
      }
    }
  }
}
```

Damage-Reducing Berry Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_source_modify_damage": [
        "$type = func_call(value_from_local_data: type)",
        "if (!$type or $move.type == $type) and ($move.type == normal or func_call(type_modifier_against_target: $move $target) > 0) and !func_call(move_hit_data_has_flag_against_target: $move $target hitsubstitute):",
        [
          "if func_call(eat_item: $target):",
          ["log_activate: with_target use_source weaken", "return $damage / 2"]
        ]
      ]
    }
  }
}
```

</details>

Damage-reducing berries are consumed when hit by a move of a specific type (or any move for the Enigma Berry). They reduce the damage taken by 50%. The reduction occurs during the damage calculation phase.

### #16 - White Herb

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_update": [
        "$activate = false",
        "foreach $stat in func_call(boostable_stats):",
        ["if func_call(get_boost: $mon.boosts $stat) < 0:", ["$activate = true"]],
        "if $activate:",
        ["use_item: $mon"]
      ],
      "on_use": ["clear_negative_boosts: $mon"]
    }
  }
}
```

</details>

White Herb is consumed and clears all stat drops whenever possible. It is not particularly complex (but it is relatively complex compared to other items!).

### #15 - Sticky Barb

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_residual": ["damage: $target expr($target.base_max_hp / 8)"],
      "on_hit": [
        "require $source != $target else return",
        "require !$source.item else return",
        "require func_call(move_makes_contact: $move) else return",
        "# Make sure we can set the item first.",
        "require func_call(set_item: $source $target.item dry_run) else return",
        "require !!assign($item = func_call(take_item: $target)) else return",
        "set_item: $source $item"
      ]
    }
  }
}
```

</details>

Sticky Barb deals damage to the holder at the end of each turn. The gimmick is that when the holder is hit by a contact move, it passes the item on to its attacker.

### #14 - Red Card

_Introduced: Generation 5_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_after_move_secondary_effects": [
        "require $source != $target else return",
        "require $source.hp != 0 else return",
        "require $target.hp != 0 else return",
        "require $move.category != status else return",
        "require $source.active else return",
        "require func_call(can_switch: $source.player) else return",
        "require !$source.force_switch else return",
        "require !$target.force_switch else return",
        "if func_call(use_item: $target):",
        ["force_switch: $source"]
      ]
    }
  }
}
```

</details>

Red Card forces an attacker to switch when they hit into a Pokémon holding this item.

### #13 - Mega Stones and Z-Crystals

_Introduced: Generation 6_

<details>
<summary><i>Code</i></summary>

Gengarite:

```json
{
  "effect": {
    "delegates": ["condition:untakablebyspeciesitembase"],
    "local_data": {
      "values": {
        "species": "gengar"
      }
    }
  }
}
```

Normalium Z:

```json
{
  "effect": {
    "delegates": ["condition:untakableitembase"]
  }
}
```

Untakable By Species Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_take_item": [
        "$base_species = func_call(base_species: $target)",
        "require $base_species != func_call(value_from_local_data: species)"
      ]
    }
  }
}
```

Untakable Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_take_item": ["return false"]
    }
  }
}
```

</details>

Mega Stones and Z-Crystals activate special battle mechanics. Mega Stones allow a particular Pokémon to Mega Evolve, and Z-Crystals allow a Pokémon (sometimes of a particular species) to transform a move into a Z-Move.

The core battle engine does all the heavy lifting for allowing and activating these mechanics based on the Pokémon's item. The only thing we implement in fxlang is that these items cannot be taken away from the holder.

### #12 - Red Orb and Blue Orb

_Introduced: Generation 6_

<details>
<summary><i>Code</i></summary>

Red Orb:

```json
{
  "effect": {
    "delegates": ["condition:untakablebyspeciesitembase", "condition:primalreversionitembase"],
    "local_data": {
      "values": {
        "species": "groudon",
        "primalforme": "groudonprimal"
      }
    }
  }
}
```

Untakable By Species Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_take_item": [
        "$base_species = func_call(base_species: $target)",
        "require $base_species != func_call(value_from_local_data: species)"
      ]
    }
  }
}
```

Primal Reversion Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_switch_in": [
        "$base_species = func_call(base_species: $mon)",
        "if !$mon.transformed and $base_species == func_call(value_from_local_data: species):",
        ["primal_reversion: $mon func_call(value_from_local_data: primalforme)"]
      ]
    }
  }
}
```

</details>

Red Orb and Blue Orb are conceptually similar to Mega Stones, except they activate Primal Reversion as soon as the holder enters the field. Primal Reversion is functionally just a forme change with a special name.

### #11 - Forme-Changing Items

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

Griseous Orb:

```json
{
  "effect": {
    "delegates": ["condition:untakablebyspeciesitembase"],
    "callbacks": {
      "on_source_base_power": [
        "$base_species = func_call(base_species: $source)",
        "if $base_species == giratina and ($move.type == ghost or $move.type == dragon):",
        ["return $base_power * 6/5"]
      ]
    },
    "local_data": {
      "values": {
        "species": "giratina"
      }
    }
  }
}
```

Untakable By Species Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_take_item": [
        "$base_species = func_call(base_species: $target)",
        "require $base_species != func_call(value_from_local_data: species)"
      ]
    }
  }
}
```

Giratina:

```json
{
  "effect": {
    "callbacks": {
      "on_switch_in": {
        "order": 1,
        "program": ["run_event_on_mon_species: Update"]
      },
      "on_update": [
        "$base_species = func_call(base_species: $mon)",
        "require $base_species == giratina else return",
        "if $mon.item == griseousorb or $mon.item == griseouscore:",
        ["forme_change: $mon giratinaorigin permanent"],
        "else if $mon.species != giratina:",
        ["forme_change: $mon giratina permanent"]
      ]
    }
  }
}
```

</details>

Several items can change the forme of its holder:

- Adamant Crystal transforms Dialga into its Origin forme.
- Lustrous Globe transforms Palkia into its Origin forme.
- Griseous Orb/Core transforms Giratina into its Origin forme.
- Rusted Sword transforms Zacian into its Crowned forme.
- Rusted Shield transforms Zamazenta into its Crowned forme.
- Wellspring Mask, Hearthflame Mask, and Cornerstone Mask transform Ogerpon into its respective forme.

These items do not do much on their own: they may boost move power for their respective species, and they are generally untakable. The forme-changing functionality of these items is typically implemented _outside_ of a battle. However, to enforce consistency, each _species_ has event callbacks ensuring the forme is consistent with the item. For example, Giratina's `Update` event transforms into Origin forme or Altered forme depending on if it is holding the Griseous Orb (or Griseous Core).

The reason we implement the forme-changing functionality on the species is because if a Giratina Origin forme enters a battle without its item, it needs to be reverted. The only place this effect could activate is on the species itself (formes typically have a delegate effect to the base species).

These forme validation events should really never happen in battle, but they are interesting to think about.

### #10 - Flavorful Berries

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

Figy Berry:

```json
{
  "effect": {
    "delegates": ["condition:eatberrybyhealthitembase", "condition:healandconfusebytasteitembase"],
    "callbacks": {
      "on_player_try_use_item": ["require !$target.exited and $target.hp < $target.max_hp"],
      "on_player_use": ["eat_given_item: $mon $this.id"],
      "on_try_eat_item": ["require $target.can_heal"]
    },
    "local_data": {
      "values": {
        "stat": "atk"
      }
    }
  }
}
```

Eat Berry by Health Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_update": ["if $mon.hp <= $mon.berry_eating_health:", ["eat_item: $mon"]]
    }
  }
}
```

Heal and Confuse by Taste Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_eat": [
        "heal: $mon expr($mon.base_max_hp / 3)",
        "if $mon.true_nature.drops == func_call(value_from_local_data: stat):",
        ["add_volatile: $mon confusion"]
      ]
    }
  }
}
```

</details>

Some berries heal one-third of the consumer's HP (more than a standard berry such as the Sitrus Berry) with the cost of potentially confusing the Pokémon if it dislikes the flavor. For example, the Figy Berry confuses a Pokémon if it does not like the spicy flavor.

The flavor a Pokémon dislikes depends on which stat is reduced by its "true nature." Starting in Generation 8, Pokémon could change their effective nature (e.g., which stats are boosted and reduced by the nature). However, the flavor tendency of the Pokémon cannot change. This means the battle engine must also track a Pokémon's true nature for the effect of these berries to work correctly.

### #9 - Repeat Ball

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_modify_catch_rate": [
        "$base_species = func_call(base_species: $target)",
        "if func_call(has_species_registered: $source.player $base_species):",
        ["return $catch_rate * 7/2"]
      ]
    }
  }
}
```

</details>

The Repeat Ball makes a Pokémon easier to catch if the trainer has caught it before. Many Poké Balls have unique effects, but the Repeat Ball is the most unique compared to anything else in the rest of the battle engine. To implement the Repeat Ball accurately, the battle engine needs an understanding of the player's Pokédex.

To support this, when creating a battle, a player can be created with a set of species registered in their Pokédex. In theory, this does not truly need to be the whole Pokédex; it could just contain species in the battle that are registered.

### #8 - Protective Pads and Heavy-Duty Boots

_Introduced: Generation 7_

<details>
<summary><i>Code</i></summary>

Protective Pads:

```json
{
  "effect": {
    "callbacks": {
      "is_contact_proof": ["return true"]
    }
  }
}
```

Heavy-Duty Boots:

```json
{
  "effect": {
    "callbacks": {
      "is_immune_to_entry_hazards": ["return true"]
    }
  }
}
```

</details>

Protective Pads and Heavy-Duty Boots are implemented the same way: they simply set some state property for the holder which must be honored where applicable. Protective Pads mark the holder as "contact-proof," which is checked internally by the function that checks if a move makes contact. Heavy-Duty Boots mark the holder as immune to entry hazards, which must be honored by each effect that considers itself an entry hazard.

The use of these state event callbacks is purely designed to avoid hard-coding these item names in multiple places. If the names of these items must change, or if another item must supply the same effect, they simply hook into the same state event.

### #7 - Metronome

_Introduced: Generation 4_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_start": ["add_volatile: $target $this.id"],
      "on_end": ["remove_volatile: $target $this.id"]
    }
  },
  "condition": {
    "callbacks": {
      "on_start": ["$effect_state.counter = 0"],
      "on_try_move": {
        "priority": -10,
        "program": [
          "# Moves that call other moves are ignored.",
          "require !func_call(move_has_flag: $move callsmove) else return",
          "$in_charge_move = func_call(has_volatile: $user twoturnmove)",
          "if $effect_state.last_move != $move.id:",
          ["$effect_state.counter = 0"],
          "else if $user.move_last_turn_succeeded or $in_charge_move:",
          ["$effect_state.counter = $effect_state.counter + 1"],
          "$effect_state.last_move = $move.id"
        ]
      },
      "on_modify_damage": [
        "$counter = func_call(min: $effect_state.counter 5)",
        "return $damage * ($counter + 5) / 5"
      ]
    }
  }
}
```

</details>

The Metronome item boosts the damage dealt by moves used consecutively by the holder. The item gives the holder a volatile status that keeps a counter each time a move is used consecutively.

### #6 - Leppa Berry

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_player_try_use_item": [
        "require $target.hp != 0 and $input.move.is_defined",
        "$index = func_call(move_slot_index: $target $input.move)",
        "require $index.is_defined",
        "$move_slot = func_call(move_slot_at_index: $target $index)",
        "require $move_slot.is_defined and $move_slot.pp < $move_slot.max_pp"
      ],
      "on_player_use": [
        "require $input.move.is_defined else return",
        "restore_pp: $mon $input.move 10"
      ],
      "on_update": [
        "require $mon.hp != 0 else return",
        "foreach $move_slot in $mon.move_slots:",
        ["if $move_slot.pp == 0:", ["eat_item: $mon", "return"]]
      ],
      "on_eat": [
        "foreach $move_slot in $mon.move_slots:",
        ["if $move_slot.pp == 0:", ["restore_pp: $mon $move_slot.id 10", "return"]]
      ]
    }
  }
}
```

</details>

A Leppa Berry restores 10 PP to a move with 0 PP when consumed. Its effect in a battle is fairly simple and just requires the event callback to loop over the holder's move slots to find a move to restore.

When used from the player's bag, the Leppa Berry restores 10 PP to the selected move. This requires the battle engine to accept and record the move selected by the player when selecting to use the move.

### #5 - Revive

_Introduced: Generation 1_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_player_try_use_item": ["require $target.fainted"],
      "on_player_use": ["revive: $mon expr($mon.max_hp / 2)"]
    }
  }
}
```

</details>

A Revive can be used from the player's bag on a fainted Pokémon to revive them with 50% of their HP. This item is not complex, but it requires us to implement reviving in the battle engine. Before Generation 9 introduced Revival Blessing, the Revive item was the _only_ way to revive a Pokémon. Given bag items are not applicable in competitive play, this mechanic only applied to single-player battles for a long time.

### #4 - Utility Umbrella

_Introduced: Generation 8_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "suppress_mon_weather": [
        "if $field.weather.is_defined and ($field.weather.is_raining or $field.weather.is_sunny):",
        ["return true"]
      ],
      "on_start": ["run_event: WeatherChange"],
      "on_end": ["run_event: WeatherChange"]
    }
  }
}
```

</details>

The Utility Umbrella suppresses the effects of sunny and rainy weather for the holder.

As mentioned when discussing complex moves and abilities, suppression is a complex topic. The Utility Umbrella is particularly interesting for how it interacts with other effects. The Utility Umbrella can suppress the weather for a single Pokémon (but not others, which means the battle engine must track per-Pokémon weather). Then, the Utility Umbrella itself can be suppressed (e.g., with Embargo).

This sort of double negation is odd, and it is the primary reason the core battle engine supports suppression at the effect level. When generating the list of effects that should run for a given callback, the battle engine considers suppression. Thus, if weather is suppressed for a Pokémon, event callbacks on the weather will not run against the Pokémon at all.

### #3 - Ability Shield

_Introduced: Generation 9_

<details>
<summary><i>Code</i></summary>

```json
{
  "effect": {
    "callbacks": {
      "on_before_switch_in": [
        "# Abilities start before items, so we must start this item before an ability tries to start and thinks it is suppressed.",
        "start_item: silent"
      ],
      "on_start": [
        "# Items cannot modify ability suppression, so we use a condition.",
        "add_volatile: $target $this.id"
      ],
      "on_end": ["remove_volatile: $target $this.id"],
      "on_set_ability": [
        "if $effect.id != trace or !$effect_state.logged_for_trace:",
        ["log_block: with_target", "$effect_state.logged_for_trace = true"],
        "return stopfail"
      ],
      "on_try_hit": [
        "if $move.id == gastroacid:",
        ["if $report:", ["log_block: with_target"], "return stopfail"]
      ]
    }
  },
  "condition": {
    "callbacks": {
      "suppress_mon_ability": {
        "priority": 1,
        "program": ["return false"]
      }
    }
  }
}
```

</details>

An Ability Shield protects the holder from having its ability changed or suppressed.

The ability suppression override is implemented as a volatile status, because internally, item event callbacks are not checked when the battle engine checks for ability suppression. Otherwise, the Ability Shield simply hooks into the appropriate event callbacks for blocking other effects.

This item can have some quite complex interactions with ability-suppressing effects.

### #2 - Choice Items

_Introduced: Generation 3_

<details>
<summary><i>Code</i></summary>

Choice Band:

```json
{
  "effect": {
    "delegates": ["condition:choicelockingitembase"],
    "callbacks": {
      "on_modify_atk": ["require !$target.dynamaxed else return", "return $atk * 3/2"]
    }
  }
}
```

Choice Locking Item Base:

```json
{
  "condition": {
    "callbacks": {
      "on_start": ["remove_volatile: $target choicelock"],
      "is_choice_locked": ["return true"],
      "on_use_move": ["add_volatile: $user choicelock"]
    }
  }
}
```

Choice Lock:

```json
{
  "condition": {
    "callbacks": {
      "on_start": [
        "require $target.active_move.is_defined",
        "$effect_state.move = $target.active_move.id"
      ],
      "on_try_end": [
        "# Ensure additional choice lock reasons are also removed.",
        "require !$target.is_choice_locked"
      ],
      "on_before_move": [
        "if !$user.effective_item and $move.id != $effect_state.move and $move.id != struggle:",
        ["log_fail: $user from_effect", "return stopfail"]
      ],
      "on_disable_move": [
        "if !$mon.is_choice_locked or !func_call(has_move: $mon $effect_state.move):",
        ["remove_volatile: $mon $this.id", "return"],
        "foreach $move_slot in $mon.move_slots:",
        ["if $move_slot.id != $effect_state.move:", ["disable_move: $mon $move_slot.id"]]
      ]
    }
  }
}
```

</details>

Choice items boost a single stat of its holder by 50%, at the cost of locking them into the same move until they switch out (or the item's effect somehow ends).

Choice items are implemented by setting both a choice-locked state on the Pokémon and adding a volatile status for the choice lock itself. The choice lock volatile status is only removed if _all_ reasons for being choice locked are also removed.

Choice locking is implemented this way because the ability Gorilla Tactics works exactly the same as the Choice Band item. Choice Band and Gorilla Tactics can stack: they both put the Pokémon into the choice lock state. If the item is removed but the ability is not, the Pokémon must remain choice locked (and same if the ability is removed but the item is not). This duplication requires us to keep track of what effects are putting the Pokémon in the choice lock state.

### #1 - Eject Button and Eject Pack

_Introduced: Generation 5_

<details>
<summary><i>Code</i></summary>

Eject Button:

```json
{
  "effect": {
    "callbacks": {
      "on_after_move_secondary_effects": [
        "require $source != $target else return",
        "require $target.hp != 0 else return",
        "require $move.category != status else return",
        "require !func_call(move_has_flag: $move future) else return",
        "require func_call(can_switch: $target.player) else return",
        "require !$target.force_switch else return",
        "require !$target.being_called_back else return",
        "require !$target.is_away_from_field else return",
        "foreach $mon in func_call(all_active_or_exited_mons):",
        ["require !$mon.being_called_back else return"],
        "$effect_state.activator = $source",
        "use_item: $target"
      ],
      "on_use": [
        "$activator = $effect_state.activator",
        "switch_out: $mon",
        "if $activator.is_defined:",
        ["set_needs_switch: $activator false"]
      ]
    }
  }
}
```

Eject Pack:

```json
{
  "effect": {
    "callbacks": {
      "on_after_boost": [
        "$activate = false",
        "foreach $boost in func_call(boostable_stats):",
        ["if func_call(get_boost: $boosts $boost) < 0:", ["$activate = true", "break"]],
        "require $activate else return",
        "require func_call(can_switch: $target.player) else return",
        "require !$target.force_switch else return",
        "require !$target.being_called_back else return",
        "require !$target.is_away_from_field else return",
        "foreach $mon in func_call(all_active_or_exited_mons):",
        ["require !$mon.being_called_back else return"],
        "$effect_state.activator = $source",
        "use_item: $target"
      ],
      "on_use": [
        "$activator = $effect_state.activator",
        "switch_out: $mon",
        "if $activator.is_defined:",
        ["set_needs_switch: $activator false"]
      ]
    }
  }
}
```

</details>

Eject Button and Eject Pack force the holder to switch out immediately. Eject Button activates when its holder is hit by a damaging move. Eject Pack activates when its holder receives a stat drop.

As mentioned with the Emergency Exit ability, switching a Pokémon out at an arbitrary point in the battle is very complex. Eject Button is not nearly as complex because it is still scoped only to move damage (similar to Berserk's activation). Additionally, both of these items must check that no other Pokémon is switching out at the same time. Only one Eject Button and/or Eject Pack is allowed to activate at a time.

## Conclusion

Pokémon battles are highly complex, with a ton of niche interactions that can occur. The highly dynamic nature of Pokémon battles is what has made them continue to be thrilling over the last 30 years.

I hope looking at some of the most complex moves, abilities, and items (at least from the perspective of my battle engine, `battler`) has been interesting to see just how deep some of the battle engine mechanics can go. As Pokémon continues to unveil new mechanics and effects, I hope to continue to expand `battler` to support everything Pokémon battles have to offer and more!
