The battle log is the primary output of `battler`. It describes public effects and actions taken in the battle.

The `battle-state` crate (specifically, the `BattleState` type in `state.rs`) is designed to read this battle log and track the battle state from a single player's point of view. This code may be helpful for understanding some nuances. However, it can also be helpful to have the ability to understand what a log means at face value.

## Reading a Log Entry

A single log entry is delimited into multiple fields by a pipe `|`. The first field is ALWAYS the title, describing what happened.

For example,

- `move` - A move was used.
- `switch` - A Mon switched in.
- `damage` - A Mon took damage.
- `faint` - A Mon fainted.
- `activate` - An effect activated.
- `start` - An effect started.
- `end` - An effect ended.

All other fields after the title are descriptors. If a descriptor contains a colon `:`, then it is a key value pair delimited by the colon. Otherwise, it is simply a flag.

For example,

- `mon:Bulbasaur,player-1,1` - The Mon receiving the action is `Bulbasaur,player-1,1`.
- `name:Tackle` - The name of the effect is `Tackle`.
- `from:move:Copycat` - The effect originated from the effect `move:Copycat`.

### Mons

Mons are identified in the form `name,player,position`, where `position` is the Mon's "active position" relative to its player. In extremely rare scenarios when the referenced Mon is inactive, `position` can be omitted.