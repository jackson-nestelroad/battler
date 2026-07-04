# Battle Engine Logs

This document describes all the log types and structures output by the `battler` battle engine. 

---

## Log Entry Format

A committed battle log entry is represented as a single line of pipe-separated (`|`) strings. The line starts with the log entry **title** (representing the action or event type), followed by one or more parameters or flags.

$$\text{title} \mid \text{key}_1\text{:value}_1 \mid \text{key}_2\text{:value}_2 \mid \text{flag}_1 \mid \dots$$

- **Key-Value Attribute**: Formatted as `key:value`.
- **Flag**: A single word with no colon or value (e.g. `shiny`, `residual`, `noanim`), indicating the presence of a boolean state.

---

## Common Serialized Types

### `MonPositionDetails`
Used to identify a specific Mon's active position on the field.
- **Format**: `Name,player_id,side_position` (e.g. `Pikachu,p1,1`)
- **Format (when inactive/unpositioned)**: `Name,player_id` (e.g. `Pikachu,p1`)

### `ActiveMonDetails`
A structured entry used to record all details of a Mon when it switches in or undergoes a species change. It serializes the following fields into the log entry:
- `player` (string, required): Player ID.
- `position` (integer, required): Active position on the side (1-based index).
- `name` (string, required): The public nickname of the Mon (masked by illusions if active).
- `health` (string, required): The Mon's health (e.g. `100/100` or `312/312`). This is represented as a percentage/public base for public views or actual HP for private views.
- `status` (string, optional): Current status condition ID (omitted if none).
- `tera` (string/Type, optional): Current Tera type (omitted if not terastallized).
- `species` (string, required): The species name.
- `level` (integer, required): Mon's level.
- `gender` (string, required): The gender of the Mon.
- `shiny` (flag, optional): Added if the Mon is shiny.

---

## Log Types Catalog

### 1. Setup & Flow Logs

#### `info`
Provides metadata about the battle parameters, format, rules, and field environment.
- **Required fields**: One of the following:
  - `battletype:Type` (e.g. `battletype:Single`)
  - `environment:Env` and `time:Time` (e.g. `environment:Grass|time:Day`)
  - `rule:RuleText` (e.g. `rule:Sleep Clause Mod: Limit one foe put to sleep`)
- **Examples**:
  - `info|battletype:Single`
  - `info|environment:Grass|time:Day`
  - `info|rule:Sleep Clause Mod: Limit one foe put to sleep`

#### `side`
Registers a side participating in the battle.
- **Required fields**:
  - `id:SideIndex` (integer, side index)
  - `name:Name` (string, side name)
- **Example**: `side|id:0|name:Player 1`

#### `player`
Registers a player in the battle, associated with their side and player position index.
- **Required fields**:
  - `id:PlayerId` (string, player ID)
  - `name:PlayerName` (string, player display name)
  - `side:SideIndex` (integer, side index)
  - `position:PositionIndex` (integer, player position index on their side)
- **Example**: `player|id:p1|name:Jackson|side:0|position:0`

#### `teamsize`
Logs a player's starting team size at the beginning of the battle.
- **Required fields**:
  - `player:PlayerId` (string)
  - `size:Size` (integer)
- **Example**: `teamsize|player:p1|size:6`

#### `teampreviewstart`
Indicates that the team preview phase has started.
- **Required fields**: None.
- **Example**: `teampreviewstart`

#### `mon`
Logs a Mon in a player's team during team preview.
- **Required fields**:
  - `player:PlayerId` (string)
  - `species:SpeciesName` (string)
  - `level:Level` (integer)
  - `gender:Gender` (string)
- **Optional fields / flags**:
  - `shiny` (flag)
- **Example**: `mon|player:p1|species:Pikachu|level:50|gender:M`

#### `teampreview`
Indicates the settings or end of the team preview.
- **Optional fields**:
  - `pick:PickedSize` (integer, the number of Mon the player must pick)
- **Examples**:
  - `teampreview`
  - `teampreview|pick:3`

#### `battlestart`
Signals the official start of the battle (switching out of team preview).
- **Required fields**: None.
- **Example**: `battlestart`

#### `turn`
Logs the beginning of a new turn.
- **Required fields**:
  - `turn:TurnNumber` (integer)
- **Example**: `turn|turn:1`

#### `time`
Logs the clock value when the battle resumes after being paused (due to a request). This is logged instead of `continue` if the `log_time` engine option is enabled.
- **Required fields**:
  - `value:TimeString` (string)
- **Example**: `time|value:120`

#### `continue`
Logs that the battle is resuming after being paused (due to a request). This is logged unless the `log_time` engine option is enabled (which logs `time` instead).
- **Required fields**: None.
- **Example**: `continue`

#### `residual`
Indicates the end of the residual (end-of-turn) phase, after all residual events for that turn have been processed.
- **Required fields**: None.
- **Example**: `residual`

#### `turnlimit`
Indicates that the maximum turn limit has been reached.
- **Required fields**: None.
- **Example**: `turnlimit`

#### `maxsidelength`
Logs the maximum active side length when uneven sides are allowed.
- **Required fields**:
  - `length:MaxSideLength` (integer)
- **Example**: `maxsidelength|length:2`

#### `win`
Logs the winner of the battle.
- **Required fields**:
  - `side:SideIndex` (integer)
- **Example**: `win|side:0`

#### `tie`
Logs a tie-game end state.
- **Required fields**: None.
- **Example**: `tie`

#### `split`
Indicates that the following logs are split into private and public versions.
- **Required fields**:
  - `side:SideIndex` (integer, which side receives the private log version)
- **Example**: `split|side:0`

---

### 2. Mon Action & State Logs

#### `ability` / `abilityend`
Logs ability activations or removal/suppression.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `ability:AbilityName` (string)
- **Optional fields**:
  - `from:EffectName` (cause of trigger/removal)
  - `of:MonPositionDetails` (source Mon)
- **Example**: `ability|mon:Gyarados,p1,1|ability:Intimidate`

#### `activate`
Logs the activation of a status, condition, effect, or clause.
- **Optional fields**:
  - `mon:MonPositionDetails`
  - `move:MoveName`
  - `condition:ConditionName`
  - `clause:ClauseName`
  - `sides:SideIndexList`
  - `from:EffectName`
- **Optional flags**:
  - `broken` (e.g. protection broken)
  - `confusion` (e.g. self-hurt from disobedience)
  - `damage`
  - `tough`
  - `weaken`
- **Examples**:
  - `activate|move:Splash`
  - `activate|mon:Mew,player-2,1|condition:Must Recharge`
  - `activate|mon:Infernape,player-2,1|condition:Break Protect|broken`

#### `addedtype`
Logs an additional type appended to a Mon's current typing (e.g. Forest's Curse).
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `type:TypeName` (string, the added type)
- **Optional fields**:
  - `from:EffectName` (cause of the added type)
  - `of:MonPositionDetails` (source Mon)
- **Example**: `addedtype|mon:Pikachu,p1,1|type:Grass`

#### `addpseudoweather` / `removepseudoweather`
Logs field-wide pseudo-weathers (e.g. Trick Room, Gravity) being created or removed.
- **Required fields**:
  - `condition:ConditionName` (string)
- **Optional fields**:
  - `from:EffectName` (cause of the pseudo-weather)
  - `of:MonPositionDetails` (source Mon)
- **Example**: `addpseudoweather|condition:Trick Room|from:move:Trick Room`

#### `addsidecondition` / `removesidecondition`
Logs side conditions (e.g. entry hazards, screens) being added or removed.
- **Required fields**:
  - `side:SideIndex` (integer)
  - `condition:ConditionName` (string, condition ID/name)
- **Optional fields**:
  - `from:EffectName` (the cause of the condition)
  - `of:MonPositionDetails` (the source Mon)
- **Example**: `addsidecondition|side:0|condition:Spikes|from:move:Spikes|of:Cloyster,p2,1`

#### `addslotcondition` / `removeslotcondition`
Logs slot-based conditions (e.g. Wish, Future Sight) applied to/removed from a field slot.
- **Required fields**:
  - `side:SideIndex` (integer)
  - `slot:SlotIndex` (integer)
  - `condition:ConditionName` (string)
- **Optional fields**:
  - `from:EffectName` (cause of the condition)
  - `of:MonPositionDetails` (source Mon)
- **Example**: `addslotcondition|side:0|slot:0|condition:Wish`

#### `addvolatile` / `removevolatile`
Logs volatile statuses being applied or removed.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `volatile:VolatileName` (string, the volatile condition ID/name)
- **Optional fields**:
  - `from:EffectName` (what applied/removed the volatile status)
  - `of:MonPositionDetails` (the source of the application/removal)
- **Example**: `addvolatile|mon:Pikachu,p1,1|volatile:Substitute`

#### `block`
Logs when a move or ability is blocked by an ability or item (e.g., Aroma Veil, Sweet Veil, Ability Shield).
- **Required fields**:
  - `mon:MonPositionDetails` (the target protected by the block)
- **Optional fields**:
  - `move:MoveName` (the move blocked)
  - `from:EffectName` (the ability/item cause, e.g. `from:item:Ability Shield`)
  - `ability:AbilityName` (the ability blocked)
  - `of:MonPositionDetails` (the attacker trying to apply the blocked effect)
- **Examples**:
  - `block|mon:Grafaiai,player-1,1|move:Doodle|from:item:Ability Shield`
  - `block|mon:Chespin,player-1,2|move:Taunt|from:ability:Aroma Veil`

#### `boost` / `unboost`
Logs stat boosts or drops.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `stat:StatName` (string, e.g. `atk`, `def`, `spa`, `spd`, `spe`, `accuracy`, `evasion`)
  - `by:Amount` (integer)
- **Optional fields**:
  - `from:EffectName` (cause of modification, e.g. `from:ability:Intimidate`)
  - `of:MonPositionDetails` (source Mon inflicting the boost/drop)
- **Optional flags**:
  - `max` (if boosted directly to maximum)
  - `min` (if dropped directly to minimum)
- **Example**: `unboost|mon:Pikachu,p1,1|stat:atk|by:1|from:ability:Intimidate|of:Gyarados,p2,1`

#### `cannotescape` / `escaped` / `forfeited`
Logs player escape or forfeit events.
- **Required fields**:
  - `player:PlayerId` (string)
- **Examples**:
  - `cannotescape|player:p1`
  - `escaped|player:p1`
  - `forfeited|player:p1`

#### `cant`
Logs that a Mon is unable to act.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
- **Optional fields**:
  - `from:EffectName` (the effect causing the prevention, e.g. `from:status:paralysis`)
  - `of:MonPositionDetails` (the source/inflicting Mon)
- **Example**: `cant|mon:Pikachu,p1,1|from:status:Paralysis`

#### `catch`
Logs a successful capture.
- **Required fields**:
  - `player:PlayerId` (string)
  - `item:ItemName` (string, the name of the ball item used)
  - `mon:MonPositionDetails` (the caught Mon)
  - `shakes:ShakeCount` (integer)
- **Optional flags**:
  - `critical` (indicates a critical capture attempt)
- **Example**: `catch|player:p1|item:Ultra Ball|mon:Pikachu,p2,1|shakes:4`

#### `catchfailed`
Logs a failed capture attempt.
- **Required fields**:
  - `player:PlayerId` (string)
  - `item:ItemName` (string, the name of the ball item used)
  - `mon:MonPositionDetails` (the target Mon)
  - `shakes:ShakeCount` (integer, number of ball shakes before breakout)
- **Optional flags**:
  - `critical` (indicates a critical capture attempt)
- **Example**: `catchfailed|player:p1|item:Poke Ball|mon:Pikachu,p2,1|shakes:2`

#### `catchrate`
Outputs debug information about the catch rates during capture check.
- **Required fields**:
  - `catchrate:RateString` (e.g. `120000/1044480`)
  - `shakeprobability:ProbabilityString` (e.g. `34000/65536`)
- **Example**: `catchrate|catchrate:120000/1044480|shakeprobability:34000/65536`

#### `clearboosts` / `clearnegativeboosts` / `clearpositiveboosts`
Clears boosts from a Mon.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
- **Optional fields**:
  - `from:EffectName` (cause of clearing)
  - `of:MonPositionDetails` (source of the effect)
- **Example**: `clearboosts|mon:Pikachu,p1,1|from:move:Haze`

#### `copyboosts`
Copies stat boosts from one Mon to another.
- **Required fields**:
  - `mon:MonPositionDetails` (the target receiving the copied boosts)
  - `source:MonPositionDetails` (the Mon boosts are being copied from)
- **Optional fields**:
  - `from:EffectName` (cause of copying)
  - `of:MonPositionDetails` (source of the effect)
- **Example**: `copyboosts|mon:Pikachu,p1,1|source:Charizard,p2,1|from:move:Psych Up`

#### `curestatus`
Logs that a Mon's status condition has been cured.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `status:StatusName` (string, the status cured)
- **Optional fields**:
  - `from:EffectName` (what cured the status, e.g. `from:item:Lum Berry`)
  - `of:MonPositionDetails` (the source of the cure)
- **Example**: `curestatus|mon:Pikachu,p1,1|status:Poison|from:item:Pecha Berry`

#### `damage` / `heal` / `sethp`
Logs HP adjustments on a Mon.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `health:HealthString` (the new HP representation, e.g. `80/100` or actual HP)
- **Optional fields**:
  - `from:EffectName` (the cause of the adjustment, e.g. `from:item:Leftovers`)
  - `of:MonPositionDetails` (the source causing the damage/heal)
- **Examples**:
  - `damage|mon:Pikachu,p1,1|health:120/312|from:move:Surf|of:Blastoise,p2,1`
  - `heal|mon:Pikachu,p1,1|health:150/312|from:item:Leftovers`

#### `debug`
Logs internal battle engine failures or warnings.
- **Required fields**:
  - `event:EventName` (string, the name of the failing event)
  - `error:ErrorString` (string, the error message)
- **Optional fields**:
  - `effect:EffectName` (string, name of the associated effect)
- **Example**: `debug|event:ModifyDamage|effect:Leftovers|error:Unexpected state connector`

#### `deductpp`
Logs PP deduction from a Mon's move (e.g. via Spite or pressure mechanics).
- **Required fields**:
  - `mon:MonPositionDetails`
  - `move:MoveName` (string)
  - `by:Amount` (integer)
- **Example**: `deductpp|mon:Misdreavus,player-2,1|move:Dark Pulse|by:4`

#### `didnotlearnmove`
Logs a Mon declining to learn a move.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `move:MoveName` (string)
- **Example**: `didnotlearnmove|mon:Pikachu,p1,1|move:Thunderbolt`

#### `dynamax` / `revertdynamax`
Logs a Mon dynamaxing or reverting back to normal.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
- **Optional fields**:
  - `of:MonPositionDetails` (the source, if different from target)
- **Example**: `dynamax|mon:Pikachu,p1,1`

#### `end`
Logs the end/expiration of an active status, volatile, condition, move, or ability on a Mon.
- **Required fields**:
  - `mon:MonPositionDetails`
- **Optional fields**:
  - `move:MoveName`
  - `condition:ConditionName`
  - `ability:AbilityName`
  - `of:MonPositionDetails`
- **Optional flags**:
  - `silent`
- **Examples**:
  - `end|mon:Ambipom,player-2,1|move:Heal Block`
  - `end|mon:Probopass,player-1,1|move:Magnet Rise`

#### `exp`
Logs experience points gained.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `exp:ExpAmount` (integer)
- **Example**: `exp|mon:Pikachu,p1,1|exp:1200`

#### `fail`
Logs the failure of an effect or move.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
- **Optional fields / values**:
  - `what:EffectName` (what failed, e.g. `what:heal`, `what:unboost`)
  - `from:EffectName` (what caused the failure)
  - `boosts:StatList` (comma-separated list of stats if `what:unboost` failed, e.g. `boosts:atk,def`)
- **Examples**:
  - `fail|mon:Pikachu,p1,1`
  - `fail|mon:Pikachu,p1,1|what:unboost|boosts:atk`
  - `fail|mon:Pikachu,p1,1|what:heal`

#### `faint` / `miss` / `supereffective` / `resisted` / `crit` / `ohko`
Standard move outcome markers on a target.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
- **Examples**:
  - `miss|mon:Pikachu,p1,1`
  - `supereffective|mon:Charizard,p1,1`
  - `crit|mon:Blastoise,p1,1`
  - `faint|mon:Pikachu,p1,1`

#### `fieldactivate`
Logs activation of field-wide conditions or moves (e.g. Perish Song, Teatime).
- **Optional fields**:
  - `condition:ConditionName`
  - `move:MoveName`
  - `weather:WeatherName`
- **Examples**:
  - `fieldactivate|condition:Stat Shuffle`
  - `fieldactivate|move:Perish Song`

#### `fieldstart` / `fieldend`
Logs when a field-wide condition (e.g. Electric Terrain, Trick Room) is created or ends.
- **Required fields**: One of:
  - `move:MoveName`
  - `condition:ConditionName`
- **Optional fields**:
  - `of:MonPositionDetails` (initiator)
  - `from:EffectName` (cause)
- **Examples**:
  - `fieldstart|move:Trick Room`
  - `fieldend|move:Psychic Terrain`

#### `formechange` / `mega` / `revertmega` / `primal` / `revertprimal` / `ultra` / `revertultra` / `gigantamax` / `revertgigantamax`
Logs various form modifications.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `species:SpeciesName` (string, new species/form)
- **Optional fields**:
  - `from:EffectName` (e.g. `from:item:Charizardite Y`)
  - `of:MonPositionDetails` (the source, if different from target)
- **Example**: `mega|mon:Gengar,player-2,1|species:Gengar-Mega|from:item:Gengarite`

#### `fxlang_debug`
Logs debugging output directly from fxlang evaluation contexts.
- **Required fields**:
  - `arg0`, `arg1`, `arg2` ... (string, formatted values of passed arguments)
- **Example**: `fxlang_debug|arg0:String("Damage calculated")|arg1:Integer(154)`

#### `hitcount`
Logs the total hit count for multi-hit moves.
- **Required fields**:
  - `hits:Count` (integer)
- **Example**: `hitcount|hits:5`

#### `immune`
Logs a Mon's immunity to a move or effect.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
- **Optional fields**:
  - `from:EffectName` (what was negated)
  - `of:MonPositionDetails` (the source of the effect)
- **Example**: `immune|mon:Gengar,p2,1|from:move:Earthquake`

#### `invertboosts`
Inverts all boost multipliers (positive becomes negative and vice versa).
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
- **Optional fields**:
  - `from:EffectName` (cause of inversion)
  - `of:MonPositionDetails` (source of the effect)
- **Example**: `invertboosts|mon:Malamar,player-2,1|from:move:Topsy-Turvy|of:Malamar,player-1,1`

#### `item` / `itemend`
Logs item activations or item consumption/discard.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `item:ItemName` (string)
- **Optional fields**:
  - `from:EffectName` (cause of trigger/removal)
  - `of:MonPositionDetails` (source Mon)
- **Optional flags (for `itemend` only)**:
  - `silent` (silences normal announcement)
  - `eat` (marks item as eaten/consumed)
- **Examples**:
  - `item|mon:Pikachu,p1,1|item:Light Ball`
  - `itemend|mon:Snorlax,p1,1|item:Iapapa Berry|eat`

#### `learnedmove`
Logs a Mon learning a new move.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
  - `move:MoveName` (string)
- **Optional fields**:
  - `forgot:ForgottenMoveName` (string, move forgotten to make room)
- **Example**: `learnedmove|mon:Pikachu,p1,1|move:Thunderbolt|forgot:Thunder Shock`

#### `move` / `animatemove`
Logs the execution or animation of a move.
- `move`: Normal move execution.
- `animatemove`: Forced animation-only trigger.
- **Required fields**:
  - `mon:MonPositionDetails` (the user of the move)
  - `name:MoveName` (string)
- **Optional fields**:
  - `target:MonPositionDetails` (the direct target Mon)
  - `from:EffectName` (what forced or triggered the move)
  - `spread:Positions` (semicolon-separated target list for spread moves)
- **Optional flags**:
  - `notarget` (if no targets were available)
  - `noanim` (disables default animations)
- **Example**: `move|mon:Pikachu,p1,1|name:Thunderbolt|target:Charizard,p2,1`

#### `prepare`
Logs preparation for a multi-turn move (e.g. Solar Beam charging).
- **Required fields**:
  - `mon:MonPositionDetails` (the Mon preparing the move)
  - `move:MoveName` (string)
- **Optional fields**:
  - `target:MonPositionDetails` (the move target)
- **Example**: `prepare|mon:Venusaur,p1,1|move:Solar Beam|target:Blastoise,p2,1`

#### `protectweaken`
Logs that a Mon's protect move was weakened/pierced by a Z-Move or Max Move.
- **Required fields**:
  - `mon:MonPositionDetails` (the protected Mon)
- **Example**: `protectweaken|mon:Chesnaught,player-1,1`

#### `restorepp`
Logs PP restoration for a Mon's move (e.g. via Leppa Berry).
- **Required fields**:
  - `mon:MonPositionDetails`
  - `move:MoveName` (string)
  - `by:Amount` (integer)
- **Optional fields**:
  - `from:EffectName` (cause of restoration)
- **Example**: `restorepp|mon:Pawmot,player-1,1|move:Revival Blessing|by:1|from:item:Leppa Berry`

#### `revive`
Logs that a fainted Mon has been revived.
- **Required fields**:
  - `mon:MonPositionDetails` (the revived Mon)
- **Optional fields**:
  - `from:EffectName` (the cause of the revival)
  - `of:MonPositionDetails` (the source causing the revival)
- **Example**: `revive|mon:Quaxly,player-1|from:move:Revival Blessing|of:Pawmot,player-1,1`

#### `setpp`
Logs setting a move's PP directly to a value (e.g. Grudge).
- **Required fields**:
  - `mon:MonPositionDetails`
  - `move:MoveName` (string)
  - `to:PPValue` (integer)
- **Optional fields**:
  - `from:EffectName`
  - `of:MonPositionDetails`
- **Example**: `setpp|mon:Misdreavus,player-2,1|move:Dark Pulse|to:0|from:move:Grudge|of:Misdreavus,player-1,1`

#### `sidestart` / `sideend`
Logs the start or end of a side condition (e.g., Light Screen, Mist, entry hazards).
- **Required fields**:
  - `side:SideIndex` (integer)
  - `move:MoveName` (the screen/hazard move name)
- **Optional fields**:
  - `count:LayerCount` (integer, e.g. for multiple layers of Spikes)
- **Examples**:
  - `sidestart|side:1|move:Stealth Rock`
  - `sideend|side:1|move:Light Screen`

#### `singlemove`
Logs the start of a single-move effect (e.g. Destiny Bond, Grudge, Glaive Rush).
- **Required fields**:
  - `mon:MonPositionDetails`
- **Optional fields**:
  - `move:MoveName`
- **Example**: `singlemove|mon:Misdreavus,player-1,1|move:Grudge`

#### `singleturn`
Logs the start of a single-turn effect (e.g. Protect, Roost, Focus Punch, Endure).
- **Required fields**:
  - `mon:MonPositionDetails`
- **Optional fields**:
  - `move:MoveName`
  - `of:MonPositionDetails`
  - `condition:ConditionName` (e.g. `condition:Z-Power`)
- **Example**: `singleturn|mon:Infernape,player-2,1|move:Protect`

#### `specieschange` / `replace` / `switch` / `drag` / `appear` / `switchout`
Logs Mon entry, exit, or identity/species changes.
- **Required fields for switches/replacements**: All fields in `ActiveMonDetails`.
- **Required fields for `switchout`**: `mon:MonPositionDetails`.
- **Examples**:
  - `switch|player:p1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M`
  - `switchout|mon:Pikachu,p1,1`

#### `start`
Logs the start of an effect, condition, volatile status, or ability on a Mon.
- **Required fields**:
  - `mon:MonPositionDetails`
- **Optional fields**:
  - `move:MoveName`
  - `of:MonPositionDetails` (source of the effect)
  - `count:Count` (e.g. turn or layer counts)
  - `perish:Count` (perish song countdown)
  - `from:EffectName` (cause of start)
  - `condition:ConditionName`
  - `disabledmove:MoveName`
  - `mimic:MoveName`
  - `ability:AbilityName`
  - `stat:StatName`
  - `fallen:Count`
- **Optional flags**:
  - `fatigue`
  - `residual`
  - `silent`
- **Examples**:
  - `start|mon:Magnezone,player-1,1|move:Magnet Rise`
  - `start|mon:Budew,player-1,1|condition:Perish Song|perish:3`

#### `status`
Logs the application of a status condition (e.g. Sleep, Bad Poison, Paralysis).
- **Required fields**:
  - `mon:MonPositionDetails`
  - `status:StatusName` (string)
- **Optional fields**:
  - `from:EffectName` (cause, like `from:Toxic Fumes` or `from:ability:Effect Spore`)
  - `of:MonPositionDetails` (inflictor Mon)
- **Example**: `status|mon:Pikachu,player-1,1|status:Bad Poison|from:Toxic Fumes`

#### `swap`
Swaps the position of active Mon on the field.
- **Required fields**:
  - `mon:MonPositionDetails` (the target Mon swapped)
  - `position:TargetPositionIndex` (integer, new position)
- **Optional fields**:
  - `from:EffectName` (cause of swap)
  - `of:MonPositionDetails` (source of swap)
- **Example**: `swap|mon:Pikachu,p1,1|position:2`

#### `swapboosts`
Swaps boosts between Mon.
- **Required fields**:
  - `mon:MonPositionDetails` (the target swapping boosts)
- **Optional fields**:
  - `stats:StatList` (comma-separated list of stats swapped)
  - `from:EffectName` (cause of boost swap)
  - `of:MonPositionDetails` (the other Mon involved in the swap)
- **Example**: `swapboosts|mon:Pikachu,p1,1|stats:atk,def|from:move:Guard Swap|of:Shuckle,p2,1`

#### `swapplayer`
Swaps the position of a player (usually in multiplayer team-shifting formats).
- **Required fields**:
  - `player:PlayerId` (string)
  - `position:TargetPositionIndex` (integer)
- **Example**: `swapplayer|player:p1|position:1`

#### `swapsideconditions` / `swapsidecondition`
Swaps screens, hazards, or other side conditions between sides.
- **Required fields**:
  - `side:SideIndex` (integer)
- **Required fields for `swapsideconditions`**:
  - `with:SourceSideIndex` (integer)
- **Required fields for `swapsidecondition`**:
  - `condition:ConditionName` (string)
  - `source:SourceSideIndex` (integer)
- **Optional fields**:
  - `from:EffectName` (cause)
  - `of:MonPositionDetails` (source Mon)
- **Example**: `swapsidecondition|side:0|condition:Reflect|source:1|from:move:Court Change|of:Cinderace,player-1,2`

#### `transform`
Logs a Mon transforming into another (e.g. Ditto using Transform).
- **Required fields**:
  - `mon:MonPositionDetails` (the Mon transforming)
  - `into:MonPositionDetails` (the target Mon being copied)
  - `species:SpeciesName` (string, species of target being copied)
- **Optional fields**:
  - `from:EffectName` (cause of transformation)
  - `of:MonPositionDetails` (source Mon)
- **Example**: `transform|mon:Ditto,p1,1|into:Mew,p2,1|species:Mew`

#### `typechange` / `resettypechange`
Logs type modifications or typing resets.
- **Required fields**:
  - `mon:MonPositionDetails` (the target)
- **Required fields for `typechange`**:
  - `types:TypeList` (slash-separated types, e.g. `Water/Ground`)
- **Optional fields**:
  - `from:EffectName` (cause)
  - `of:MonPositionDetails` (source Mon)
- **Example**: `typechange|mon:Pikachu,p1,1|types:Water|from:move:Soak`

#### `uncatchable`
Logs that a Mon cannot be caught.
- **Required fields**:
  - `player:PlayerId` (string)
  - `mon:MonPositionDetails` (the target Mon)
- **Optional flags**:
  - `thief` (indicates a trainer Mon cannot be stolen)
- **Example**: `uncatchable|player:p1|mon:Pikachu,p2,1`

#### `useitem`
Logs a player using an item from their bag (e.g. in PvE/wild encounters).
- **Required fields**:
  - `player:PlayerId` (string)
  - `name:ItemName` (string)
- **Optional fields**:
  - `target:MonPositionDetails` (target Mon of the item)
- **Example**: `useitem|player:p1|name:Ultra Ball|target:Mewtwo,p2,1`

#### `waiting`
Logs that one Mon is waiting for another (e.g. during pledge moves).
- **Required fields**:
  - `mon:MonPositionDetails` (the waiting Mon)
  - `on:MonPositionDetails` (the target being waited on)
- **Example**: `waiting|mon:Pikachu,p1,1|on:Charizard,p2,1`

#### `weather` / `clearweather`
Logs weather starting, continuing, or clearing.
- **Required fields for `weather`**:
  - `weather:WeatherName` (string)
- **Optional fields**:
  - `from:EffectName` (cause)
  - `of:MonPositionDetails` (source Mon)
- **Optional flags**:
  - `residual` (weather residual end-of-turn continuation)
- **Examples**:
  - `weather|weather:Rain|from:Start`
  - `weather|weather:Sandstorm|residual`
  - `clearweather`
