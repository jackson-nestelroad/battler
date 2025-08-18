# Battle Events

**Battle Events** are the fundamental way that different effects (moves, abilities, items, etc.) can extend the battle engine. This document is intended to describe the order of events, to make it easier to write new battle effects.

Note that this is not meant to be an exact description of how battles work. Many events have nuance to how and when they are applied. Thus, only a high-level overview is presented here.

## Team Validation

1. For each player:
   1. `ValidateTeam`.
   1. For each Mon:
      1. `ValidateMon`.

## Start

1. All Mons are switched in, as specified by the battle format.
1. Switch events are scheduled in speed order.

## Switch

1. `CopyVolatile` for each volatile, if applicable.
1. Switch old Mon out:
   1. If Mon is not fainted:
      1. `BeforeSwitchOut`.
      1. `SwitchOut`.
      1. Ability `End`.
      1. `Exit`.
   1. Clear volatile effects.
1. Switch new Mon in:
   1. For each Mon in speed order:
      1. `BeforeSwitchIn`.
   1. For each Mon in speed order:
      1. `SwitchIn`.
      1. `EntryHazard`.
      1. Ability `Start`.
      1. Item `Start`.

## Before Turn Start

1. `DisableMove` can selectively disable moves for each Mon.
1. `TrapMon` will set the Mon as trapped.
1. `PreventUsedItems` will prevent the Mon from using items.
1. `LockMove` affects move request if applicable.
1. `MoveTargetOverride` affects move request, per move, if applicable.

## Player Choice

### Player Chooses Switch

1. No events.

### Player Chooses Move

1. Move is compared against `LockMove`.
1. "Before turn" and "priority charge" actions are additionally scheduled onto the battle queue for the move.

### Player Chooses Escape

1. No events.

### Player Chooses Forfeit

1. No events.

### Player Chooses Item

1. `PlayerTryUseItem` checks if the item can be used against the target.

## Choice Sorting

1. Move `ModifyPriority`.
1. Move `SubPriority`.
1. Calculate Mon speed for action sorting.

## Forfeit

1. All the player's Mons are switched out.

## Player Uses Item

1. `PlayerUse`.
1. If the item is a ball:
   1. Calculate modified catch rate.
      1. `ModifyCatchRate`.
      1. If caught, the catch will be processed later.

## Mon Caught

1. Give out experience.
1. Caught Mon pExit`.
1. Ability `End`.
1. Clear volatile effects.
1. Switch out.

## Before Turn (per move)

1. Move `BeforeTurn`.

## Escape

1. `CanEscape`.
1. `ForceEscape`.
1. All the player's Mons are switched out.

## Mega Evolution

1. Not implemented.

## Priority Charge (per move)

1. Move `PriorityChargeMove`.

## Move

1. `OverrideMove`.
1. Use active move:

   1. `BeforeMove`.
      1. _(extension)_ `Flinch`.
   1. `MoveAborted` if `BeforeMove` failed.
   1. `ModifyTarget`.
   1. Use active move:

      1. `UseMove`.
      1. `RedirectTarget`.
      1. `TryMove`.
      1. `UseMoveMessage`.
      1. Mon self-destructs.
      1. Indirect move (field or side):
         1. `TryUseMove`.
         1. `PrepareHit`.
         1. `TryHitField` or `TryHitSide`.
         1. Move hit against all targets (see below).
      1. Direct move (one or more Mons):

         1. `TryUseMove`.
            1. _(extension)_ `ChargeMove`.
         1. `PrepareHit`.
            1. _(extension)_ `StallMove`.
         1. Move hit loop:

            1. Calculate number of hits.
            1. For each hit:
               1. Prepare move for each target. As soon as a step fails, the target is removed from the list.
                  1. Check invulnerability: `Invulnerability`.
                  1. Check hit: `TryHit`.
                  1. Check type immunity: `IgnoreImmunity`, `NegateImmunity`, `TypeImmunity`, consult type chart.
                  1. Check general immunity: `TryImmunity`, `Immunity`.
                  1. Check accuracy:
                     1. Modify accuracy:
                        1. OHKO moves: `IsSemiInvulnerable`, level check, accuracy check.
                        1. Else: `ModifyAccuracy`, `ModifyBoosts` (for accuracy and evasion).
                     1. `AccuracyExempt`.
               1. Move hit against all targets:
                  1. `TryHitField`, `TryHitSide`, or `TryHit` for each target Mon.
                  1. `TryPrimaryHit`.
                     1. _(extension)_ `AfterSubstituteDamage`.
                  1. Calculate damage for each target:
                     1. `MoveDamage`.
                     1. `MoveBasePower`.
                     1. `BasePower`.
                     1. `ModifyCritRatio`.
                     1. `ModifyCritChance`.
                     1. `CriticalHit`.
                     1. Calculate stats:
                        1. `ModifyBoosts`.
                        1. `ModifyAtk`, `ModifyDef`, `ModifySpA`, `ModifySpD`, or `ModifySpe`.
                     1. Apply damage modifiers:
                        1. `WeatherModifyDamage`.
                        1. Randomize base damage.
                        1. Type effectiveness: `Effectiveness`.
                        1. `ModifyDamage`.
                  1. Apply damage for each target:
                     1. Check immunity: `Immunity`.
                     1. `Damage` for final damage modifiers.
                     1. Apply drain.
                  1. Apply move effects:
                     1. Stat boosts:
                        1. `ChangeBoosts`.
                        1. `TryBoost`.
                     1. Heal:
                        1. `CanHeal`.
                        1. `TryHeal`.
                     1. Set status:
                        1. `CureStatus`.
                        1. Check immunity.
                        1. `SetStatus`.
                        1. Status `Duration`.
                        1. Status `Start`.
                        1. `AfterSetStatus`.
                     1. Add volatile:
                        1. Volatile `Restart`.
                        1. Check immunity.
                        1. `AddVolatile`.
                        1. Volatile `Duration`.
                        1. Volatile `Start`.
                        1. `AfterAddVolatile`.
                     1. Add side condition:
                        1. Condition `SideRestart`.
                        1. Condition `Duration`.
                        1. Condition `SideStart`.
                        1. `SideConditionStart`.
                     1. Add slot condition:
                        1. Condition `SlotRestart`.
                        1. Condition `Duration`.
                        1. Condition `SlotStart`.
                        1. `SideConditionStart`.
                     1. Set weather:
                        1. `ClearWeather`.
                        1. `SetWeather`.
                        1. Weather `Duration`.
                        1. Weather `FieldStart`.
                        1. `WeatherChange`.
                     1. Set terrain:
                        1. `ClearTerrain`.
                        1. `SetTerrain`.
                        1. Terrain `Duration`.
                        1. Terrain `FieldStart`.
                        1. `TerrainChange`.
                     1. Add pseudo-weather:
                        1. Pseudo-weather `FieldRestart`.
                        1. `AddPseudoWeather`.
                        1. Terrain `Duration`.
                        1. Terrain `FieldStart`.
                        1. `AfterAddPseudoWeather`.
                     1. Apply force switch if possible.
                     1. `HitField`, `HitSide`, or `Hit`.
                     1. Mon self-destructs conditionally.
                  1. Hit user for user effect once.
                  1. Apply secondary effects:
                     1. For each target: `ModifySecondaryEffects`, move hit against target with secondary context.
                     1. Note that some logic above does not apply for secondary effects, such as damage.
                  1. Force switch:
                     1. `DragOut`.
                     1. Set switch flag.
                  1. For each target that received damage:
                     1. `DamagingHit`.
                     1. `AfterHit`.
               1. `Update`.
            1. Process fainted Mons.
               1. Give out experience.
               1. `Faint`.
               1. `Exit`.
               1. Clear volatile effects.
               1. Switch out.
            1. Apply recoil damage.
            1. `Update`.
            1. `AfterMoveSecondaryEffects`.

         1. `MoveFailed`.

   1. `AfterMove`.

1. `SetLastMove`.
1. `DeductPp`.

## Remove Volatile

1. Volatile `End`.

## Set Ability

1. `SetAbility`.
1. Ability `End`.
1. Ability `Start`.

## Set Item

1. Item `End`.
1. Item `Start`.

## Use Item

1. `TryUseItem`.
1. Item `Use`.

## Eat Item

1. `TryEatItem`.
1. Item `Eat`.
1. `EatItem`.

## Take Item

1. `TakeItem`.
1. Item `End`.

## Restore PP

1. `RestorePp`.

## Giving Experience

1. Gain EVs.
1. Recalculate stats.
1. Calculate experience gain:
   1. `ModifyExperience`.
1. Schedule processing experience in battle queue.

## Gaining Experience

1. Calculate new level.
1. Schedule level up action(s) in battle queue.

## Level Up

1. Recalculate stats.
1. Increase friendship:
   1. `ModifyFriendshipIncrease`.
1. Record learnable moves.

## Learn Move

1. Update base move slots.

## Residual

1. Update speed for each active Mon.
1. `Residual` event (or `FieldResidual`, `SideResidual`, `SlotResidual`, according to the location of the effect) for each active effect on the field, in speed order.
   1. _(extension)_ `Weather`.
1. Before running the `Residual` event, the duration of the effect is subtracted by one. If duration reaches 0, the `End` event (or `FieldEnd`, `SideEnd`, `SlotEnd`, according to the location of the effect) is run instead, and the effect is removed.

## End

1. `EndBattle` for each active Mon.

## State Events

### Mons

- `IsAsleep`.
- `IsAwayFromField`.
- `IsBehindSubstitute`.
- `IsContactProof`.
- `IsGrounded`.
- `IsImmuneToEntryHazards`.
- `IsSemiInvulnerable`.
- `IsSoundproof`.
- `Types`.

### Weather

- `IsRaining`.
- `IsSnowing`.
- `IsSunny`.

### Effect

- `SuppressFieldTerrain`.
- `SuppressFieldWeather`.
- `SuppressMonAbility`.
- `SuppressMonItem`.
- `SuppressMonTerrain`.
- `SuppressMonWeather`.
