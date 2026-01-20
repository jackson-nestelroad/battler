## Background

A battle takes place on a field. A field has two sides competing against each other. Each side consists of one or more players. Each player has one or more Mons on their team. A single player can bring any number of Mons to a battle in their team.

Only a fixed number of a player's Mons can be active at one time. An active Mon is present on the field and is actively fighting. An inactive Mon, on the other hand, is not present and is stored away in the player's team.

Players on the same side work together to win the battle. A side is declared the winner when all Mons on the opposing side faint (HP drops to zero) and are unable to battle.

## Turns

Battles are turn-based. On each turn, each player can make an action for each active Mon. An action can be one of the following:

- Use a move - The Mon can use one of its moves towards one or more active Mons. A move can only be used towards a valid target. A move's valid targets can vary move to move. A move cannot be used if it is disabled or out of PP. Additionally, during a move action, additional mechanics may be selected for activation on the Mon if allowed, such as Mega Evolution, Dynamax, or Terastallization.
- Switch out - The player can switch out an active Mon for some inactive Mon on the player's team. A Mon that is already active cannot be switched in.
- Use item - The player can use an item from their bag. The item can only be used towards a valid target. An item's valid targets can vary item to item.
- Escape - The player can attempt to run away and escape the battle. Players can only escape from wild battles. An escaped player no longer participates in a battle.
- Forfeit - The player can forfeit the battle and leave immediately. Players can only forfeit trainer battles. A forfeited player no longer participates in a battle.

After all players select actions for the turn, each Mon performs their action in order. Turn order is determined by action priority (e.g., switches occur before all moves, moves with higher priority are performed first) and each Mon's speed stat (Mons with higher speed stats will move first).

## Mon Positioning

A Mon has several properties for determining its position in your team and on the field.

- Team position - The zero-based index for the Mon in its player's team. Used when switching Mon in.
- Active position - The zero-based index for the Mon in the player's active battling positions. Used to identify Mons that are battling for a single player.
- Side position - The zero-based index for the Mon on its side of the field. Used when targeting Mons with moves.

Every Mon has a team position. Only active Mons have an active position and side position.

Side positions are always numbered from left to right for Mon facing outward. Since the two sides of battles face each other, opposing side positions actually appear from right to left, since opposing Mon are facing you for battle.

For a Singles battle, only one Mon can be active on a side at a time, so both active Mons' side positions are trivially 0.

For a Doubles battle, two Mons can be active on a side at a time. The side positions appear as follows:

```
                        Side position
Opposing side:          1       0
Ally (your) side:       0       1
```

## Choice Format

Choices for each Mon in a request must follow a strict format.

### Choosing a Pass (Test Only)

A pass action is a string of the format `pass`.

You should choose to pass when the Mon's choice does not contribute to the test.

### Choosing a Move

A move action is a string of the format `move $move_index(, $target_side_position)?`. `$move_index` is the index of the selected move in the `request_data.turn.moves` array. `$target_side_position` is the side position of the move target.

`$target_side_position` must be non-empty ONLY IF the move's target type is one of the following:

- Normal
- Any
- AdjacentAlly
- AdjacentAllyOrUser
- AdjacentFoe

For all other move target types not listed above (e.g., AllAdjacent, AllAdjacentFoes, User, etc.), `$target_side_position` MUST be empty.

`$target_side_position` is a signed one-based index representing the side position being targeted. A positive number targets the opposing side, while a negative number targets the same side. Zero is never a valid target.

`$target_side_position` calculation follows the following rules:

- If targeting an ally, use `-1 * (side_position + 1)`, where `side_position` is a zero-based index numbered left to right.
- If targeting a foe, use `side_position + 1`, where `side_position` is a zero-based index numbered right to left.

The `side_position` of allies can be derived from turn request data or the battle state object. The `side_position` of foes must be derived from the battle state object (specifically, a Mon's `side_position` is its index in `side[N].active`).

You may append additional strings to the move action if you choose to activate special battle mechanics:

- Append `, mega` if you are Mega Evolving.
- Append `, dyna` if you are Dynamaxing.
- Append `, tera` if you are Terastallizing.

Below are some examples of valid move actions:

- `move 0, 1` - Use the Mon's move in the 0 index, targeting the foe Mon in `side_position` 0.
- `move 2, 2`- Use the Mon's move in the 2 index, targeting the foe Mon in `side_position` 1.
- `move 1` - Use the Mon's move in the 1 index, with no target (since the move does not allow a target to be selected).
- `move 1, -1, mega` - Use the Mon's move in the 1 index, targeting the ally Mon in `side_position` 0, and Mega Evolve.

### Choosing to Switch

A switch action is a string of the format `switch $team_position`, where `$team_position` is the value of the `team_position` field for the Mon to switch in. For example, `switch 2` switches in the 3rd Mon on your team (since `team_position` is a zero-based index).

Below are some examples of valid switch actions:

- `switch 0` - Switches to the Mon in `team_position` 0.
- `switch 2` - Switches to the Mon in `team_position` 2.

### Choosing to Use Item

An item action is a string of the format `item $item(, $target_side_or_team_position(, $additional_input)?)?`, where `$item` is the selected item ID from the bag, `$target_side_or_team_position` is the position of the target, and `$additional_input` is any additional input required for using the item.

`$target_side_or_team_position` functions similarly to `$target_side_position`. However, for items used on a Mon in your team, the value is the Mon's team position.

`$target_side_or_team_position` calculation follows the following rules:

- If targeting a Mon in your team, use `-1 * (team_position + 1)`, where `team_position` is the zero-based index of the Mon in your team.
- If targeting a foe, use `side_position + 1`, where `side_position` is a zero-based index numbered right to left.

## Choosing to Escape/Forfeit

An escape action is a string of the format `escape`. A forfeit action is a string of the format `forfeit`.

You should very rarely choose to escape or forfeit, unless you have absolutely no way to win the battle.