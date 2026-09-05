# Ability & Item Notices: State Synchronization & Attribution Specification

This document exhaustively details the desired behavior and technical implementation of **ability** and **item** start/end battle logs, tracking their effects across:
1. **`battler-state`**: The client-side state machine tracking Pokémon appearance, abilities, held items, and previous items.
2. **`battler-log-formatter`**: The client-side log formatting engine responsible for generating localized text and attributing **UI Notice Badges** (`UiNotice[]`) to the correct combatants.

---

## 1. Architectural Roles & Primitives

When the battle engine executes turns, it emits event logs with structured attributes. The client architecture maintains state and produces formatted UI representations through a coordinated pipeline:

```
                  ┌──────────────────────┐
                  │    battler Engine    │
                  └──────────┬───────────┘
                             │
                             ▼ (raw log entries: title, mon, item, source, of, etc.)
                  ┌──────────────────────┐
                  │    battler-state     │
                  │  (updates live state)│
                  │  • Appearance        │
                  │  • Volatile ability  │
                  │  • Held/prev items   │
                  └──────────┬───────────┘
                             │
                             ├──────────────────────────┐
                             │ (raw log entry)          │ (live BattleState context:
                             ▼                          │  genders, side names, teams)
                  ┌─────────────────────────────────────┴┐
                  │       battler-log-formatter          │
                  │ (depends on battler-state)           │
                  │ • Maps patterns & context variables  │
                  │ • Evaluates notice-rules.json        │
                  │ • Localized messages via i18next     │
                  │ • Emits UI notice badges (UiNotice[])│
                  └──────────────────┬───────────────────┘
                                     │
                                     ▼
                  ┌──────────────────────────────────────┐
                  │            Battle Client             │
                  │   • Formatted localized log text     │
                  │   • Prominent notice badges          │
                  └──────────────────────────────────────┘
```

1. **`battler-state`** consumes logs sequentially to maintain a live, authoritative client-side `BattleState`. It tracks what each Pokémon has revealed about itself, what volatile effects it is currently undergoing, and its item history.
2. **`battler-log-formatter`** directly depends on `battler-state`. It receives each log entry alongside the live `BattleState` (to resolve side names, genders, and active Pokémon details) and maps them into localized combat text and structured UI notice badges (`UiNotice[]`).

### Log Attributes Reference

| Attribute | Meaning | Common Usage |
| :--- | :--- | :--- |
| `mon` | The primary Pokémon subject of the log entry. | The actor switching in, using a move, acquiring an ability, or taking damage. |
| `source` | The interacting Pokémon partner or opponent in an asymmetric transfer or reveal. | The victim whose item was stolen/consumed (Pluck/Bug Bite), or the donor whose ability was copied (Trace). |
| `from` | The underlying effect causing the log. | `from:ability:Trace`, `from:move:Thief`, `from:item:Life Orb`. |
| `of` | The owner of the `from` effect when distinct from `mon`. | The Pokémon retaliating with contact damage (`from:ability:Rough Skin|of:<Mon>`). |
| `ability` / `item` | The specific ability or item name. | `"Intimidate"`, `"Sitrus Berry"`, `"Choice Band"`. |

---

## 2. The Desired State

### A. Ability State Semantics
* **Base Ability (`battle_appearance.ability`)**: The innate ability belonging to the Pokémon. Once revealed to the client, it is permanent knowledge for that Mon and must **not** be overwritten by temporary effects or donor lookups.
* **Volatile Ability (`volatile_data.ability`)**: A temporary combat ability acquired in battle (e.g. via Trace, Skill Swap, Role Play, or Lingering Aroma). When an ability ends or is suppressed (e.g. Neutralizing Gas), the volatile ability is cleared.

### B. Item State Semantics
* **Active Held Item (`item`)**: The item currently held by the Pokémon.
* **Previous Item (`previous_item`)**: The last consumed or ended item held by that Pokémon. Necessary for mechanics such as Recycle, Belch, and Poltergeist.
* **Non-Belonging Consumption (Theft / Pluck Rule)**: When Pokémon $A$ steals and consumes an item belonging to Pokémon $B$ (e.g. Pluck or Bug Bite eating a berry), **the item never belonged to $A$**.
  - $B$'s held item is cleared and recorded as $B$'s `previous_item`.
  - $A$'s held item is **strictly preserved** (e.g. $A$ continues holding its Life Orb).
  - $A$ does **not** record the stolen berry as its `previous_item`.

### C. Notice Badge Attribution Semantics
UI notice badges (`UiNotice`) display an ability or item pill next to a Pokémon name (e.g. `[Gyarados's Intimidate]`, `[The opposing Snorlax's Sitrus Berry]`).
* **Unilateral / Self**: Attributed to `mon`.
* **Asymmetric Reveal / Copy**: Attributed to both target and source when both now possess or revealed the trait (e.g. Trace, Frisk).
* **Stolen Item Consumption (`itemend` with `source`)**:
  - The log text line explicitly explains the action: `"<Attacker> stole and ate <Victim>'s <Item>!"`.
  - Any subsequent healing/boost log credits the eater.
  - The `Item` notice badge on `itemend` is attributed **exclusively to `source`** (the owner whose item was ended). The eater never held the item, so no misleading badge is shown for the eater.

---

## 3. Exhaustive Scenario Matrix

### Permutations Summary Matrix

The matrix below captures all combinations of **(Ability, Item) × (Start, End) × (With Source, Without Source / Of)**:

| Trait | Action | Target / Entity Context | Canonical Log Example | Notice Badges Emitted | State Mutations (`battler-state`) | Reference |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Ability** | **Start** | Without Source (Self / Volatile Acquisition)\*\* | `abilitystart\|mon:Cinderace,p1,1\|ability:Libero` | 1: Cinderace (`Libero`) | Cinderace volatile ability set to `Libero`; base ability untouched | [Scenario 1](#scenario-1-unilateral-ability-start--acquisition-abilitystart-without-source) |
| **Ability** | **Start** | With Source (Copy / Trace) | `abilitystart\|mon:Ralts,p1,1\|ability:Refrigerate\|source:Charmander,p2,1\|from:ability:Trace` | 2: Charmander (`Refrigerate`), Ralts (`Refrigerate`) | Ralts volatile set to `Refrigerate`, base set to `Trace`; Charmander base set to `Refrigerate` | [Scenario 2](#scenario-2-ability-copy--trace-abilitystart-with-source-and-from) |
| **Ability** | **Start** | With `of` + `from` (Contact / Transmutation) | `abilitystart\|mon:Slaking,p1,1\|ability:Lingering Aroma\|from:ability:Lingering Aroma\|of:Oinkologne,p2,1` | 2: Oinkologne (`Lingering Aroma`), Slaking (`Lingering Aroma`) | Slaking volatile set to `Lingering Aroma`; Oinkologne base set to `Lingering Aroma` | [Scenario 3](#scenario-3-ability-replacement-on-contact-abilitystart-with-from-and-of) |
| **Ability** | **End** | Without Source / Of (Unilateral End) | `abilityend\|mon:Slaking,p1,1\|ability:Truant` | 1: Slaking (`Truant`) | Slaking volatile ability cleared to `""` | [Scenario 4](#scenario-4-unilateral-ability-end-abilityend-without-source-or-of) |
| **Ability** | **End** | With External Entity (`of` + `from`)\* | `abilityend\|mon:Slaking,p1,1\|ability:Truant\|from:ability:Neutralizing Gas\|of:Weezing,p2,1` | 2: Weezing (`Neutralizing Gas`), Slaking (`Truant`) | Slaking volatile ability cleared; Weezing base set to `Neutralizing Gas` | [Scenario 5](#scenario-5-ability-suppression--removal-with-external-entity-abilityend-with-from-and-of) |
| **Item** | **Start** | Without Source (Self Reveal) | `itemstart\|mon:Squirtle,p1,1\|item:Air Balloon` | 1: Squirtle (`Air Balloon`) | Squirtle held `item` set to `Air Balloon` | [Scenario 6](#scenario-6-self-item-reveal-itemstart-without-source) |
| **Item** | **Start** | With Source (Theft / Transfer) | `itemstart\|mon:Aipom,p1,1\|item:Choice Band\|source:Tyranitar,p2,1\|from:move:Thief` | 2: Tyranitar (`Choice Band`), Aipom (`Choice Band`) | Aipom held `item` set to `Choice Band`; Tyranitar `previous_item` recorded if unknown (held item cleared by paired `itemend`) | [Scenario 7](#scenario-7-item-theft--transfer-itemstart-with-source) |
| **Item** | **End** | Without Source (Standard Consumption) | `itemend\|mon:Snorlax,p1,1\|item:Sitrus Berry\|eat` | 1: Snorlax (`Sitrus Berry`) | Snorlax held `item` cleared; `previous_item` set to `Sitrus Berry` | [Scenario 8](#scenario-8-standard-item-consumption-itemend-without-source) |
| **Item** | **End** | With Source (Stolen / Field Consumption)\*\*\* | `itemend\|mon:Scizor,p1,1\|item:Sitrus Berry\|eat\|source:Snorlax,p2,1` | 1: Snorlax (`Sitrus Berry`) | Snorlax held `item` cleared; Snorlax `previous_item` set to `Sitrus Berry`; Scizor held item **preserved** | [Scenario 9](#scenario-9-stolen-item-consumed-on-field-itemend-with-source) |

> [!NOTE]
> \* **Battle Engine Convention for `abilityend`**: In the Pokémon battle engine, `abilityend` never emits a raw `source:` key. When an external Pokémon suppresses or ends another Pokémon's ability (e.g. Neutralizing Gas, Gastro Acid, Skill Swap, Worry Seed), the engine attributes the acting Pokémon via `of:<Mon>` with `from:<Effect>`. Both forms (unilateral without `of` and interactive with `of`) are fully accounted for in the client state machine and notice badge rules.
>
> \*\* **Innate Ability Triggers vs. `abilitystart`**: In the Pokémon battle engine, innate abilities triggered passively or on switch-in (e.g. Intimidate, Drizzle, Electric Surge) emit `activate|mon:...|ability:...` or `ability|mon:...|ability:...`, which updates `battle_appearance.ability` (base ability). In contrast, `abilitystart` specifically represents an ability starting, transforming, or replacing an existing ability due to an effect (e.g. Libero, Protean, Trace, Skill Swap).
>
> \*\*\* **Battle Engine Convention for `itemend`**: In the Pokémon battle engine, berry theft/eating moves (Bug Bite, Pluck) emit `itemend|mon:<Victim>|item:<Item>|from:move:Pluck|of:<Attacker>`. Because the victim is directly named as `mon:`, standard target resolution applies cleanly to the victim. The client state machine and formatter additionally support the asymmetric `source:` format (`itemend|mon:<Attacker>|item:<Item>|source:<Victim>`), ensuring that under both formats, only the victim's item is ended and only the victim receives the notice badge.

---

### Scenario 1: Unilateral Ability Start / Acquisition (`abilitystart` without `source`)
* **Context**: Mon changes or acquires an ability on itself via an effect or move without a donor Pokémon (e.g. Cinderace's Libero, Greninja's Protean).
* **Sample Log**:
  ```text
  abilitystart|mon:Cinderace,player-1,1|ability:Libero
  ```
* **Desired State in `battler-state`**:
  - Sets `Cinderace`'s volatile ability to `Libero` (`mon.volatile_data.record_ability("Libero")`).
  - Base appearance ability remains permanent and untouched.
  - Active ability resolves to `Some("Libero")`.
* **Desired State in `battler-log-formatter`**:
  - Emits 1 notice:
    - `{ type: "Ability", name: "Libero", mon: "Cinderace's" }`
* **Implementation Details**:
  - **`battler-state`** ([`log_handler.rs`](./battler-state/src/state/log_handler.rs)): Sets `mon.volatile_data.record_ability(...)`.
  - **`battler-log-formatter`** ([`notice-rules.json`](./js-clients/battler-log-formatter/src/config/notice-rules.json)): Matched by rule `titleIn: ["ability", "abilitystart", "abilityend", "activate"]` with `monResolution: "targetFirst"`.

---

### Scenario 2: Ability Copy / Trace (`abilitystart` with `source` and `from`)
* **Context**: Mon copies an opponent's ability (e.g. Trace, Imposter, Receiver).
* **Sample Log**:
  ```text
  abilitystart|mon:Ralts,player-1,1|ability:Refrigerate|source:Charmander,player-2,1|from:ability:Trace
  ```
* **Desired State in `battler-state`**:
  - **Target (`Ralts`)**: Base ability recorded as `Trace` (from `from:ability:Trace`); volatile ability set to `Refrigerate`.
  - **Source (`Charmander`)**: Base ability recorded as `Refrigerate` (from `ability`). If Charmander already had a known ability (e.g. Blaze), it is not overwritten.
* **Desired State in `battler-log-formatter`**:
  - Emits **2** notices:
    1. `{ type: "Ability", name: "Refrigerate", mon: "The opposing Charmander's" }` (source revealing its ability)
    2. `{ type: "Ability", name: "Refrigerate", mon: "Ralts's" }` (target acquiring the ability)
* **Implementation Details**:
  - **`battler-state`** ([`log_handler.rs`](./battler-state/src/state/log_handler.rs)): Target sets volatile ability to `Refrigerate` and base appearance to `Trace`. Source sets base appearance to `Refrigerate` only if not previously known (`mon.primary().battle_appearance.ability.known().is_none()`).
  - **`battler-log-formatter`** ([`notice-rules.json`](./js-clients/battler-log-formatter/src/config/notice-rules.json)):
    - Rule (`titleIn: ["abilitystart"], hasContext: "ABILITY", hasSource: true`) emits the source notice (`sourceOnly`).
    - Rule (`titleIn: ["ability", "abilitystart", "abilityend", "activate"]`) emits the target notice (`targetFirst`).

---

### Scenario 3: Ability Replacement on Contact (`abilitystart` with `from` and `of`)
* **Context**: Attacker strikes a target and its ability is transformed by contact (e.g. Lingering Aroma, Mummy).
* **Sample Log**:
  ```text
  abilitystart|mon:Slaking,player-1,1|ability:Lingering Aroma|from:ability:Lingering Aroma|of:Oinkologne,player-2,1
  ```
* **Desired State in `battler-state`**:
  - `Slaking`'s volatile ability becomes `Lingering Aroma`.
  - `Oinkologne`'s base ability confirmed as `Lingering Aroma`.
* **Desired State in `battler-log-formatter`**:
  - Emits **2** notices:
    1. `{ type: "Ability", name: "Lingering Aroma", mon: "The opposing Oinkologne's" }` (contact source)
    2. `{ type: "Ability", name: "Lingering Aroma", mon: "Slaking's" }` (target receiving ability)
* **Implementation Details**:
  - **`battler-state`**: `record_effect_from_mon` resolves `from:ability` with `of`, learning Oinkologne's base appearance ability (if not known). `modify_state_from_effect` records Lingering Aroma as Slaking's volatile ability.
  - **`battler-log-formatter`**: The source effect rule (`hasSourceEffectType: "Ability"`) with `hasOf: true` (`sourceFirst`) emits the notice for `Oinkologne`, while the general ability rule (`targetFirst`) emits the notice for `Slaking`.

---

### Scenario 4: Unilateral Ability End (`abilityend` without `source` or `of`)
* **Context**: Mon's temporary or innate ability ends or reverts passively without an external suppressing Pokémon (e.g. Truant loafing turn ends, Forme Change revert, or fainting).
* **Sample Log**:
  ```text
  abilityend|mon:Slaking,player-1,1|ability:Truant
  ```
* **Desired State in `battler-state`**:
  - `Slaking`'s volatile ability is cleared (`mon.volatile_data.record_ability(String::default())`).
  - Active ability reverts to innate base appearance ability.
* **Desired State in `battler-log-formatter`**:
  - Emits **1** notice:
    - `{ type: "Ability", name: "Truant", mon: "Slaking's" }`
* **Implementation Details**:
  - **`battler-state`** ([`log_handler.rs`](./battler-state/src/state/log_handler.rs)): Clears volatile ability and records activated ability on target mon.
  - **`battler-log-formatter`** ([`notice-rules.json`](./js-clients/battler-log-formatter/src/config/notice-rules.json)): Matched by rule `titleIn: ["ability", "abilitystart", "abilityend", "activate"]` with `monResolution: "targetFirst"`.

---

### Scenario 5: Ability Suppression / Removal with External Entity (`abilityend` with `from` and `of`)
* **Context**: An ability ends or is suppressed by an outside effect belonging to another combatant (e.g. Neutralizing Gas, Gastro Acid).
* **Sample Log**:
  ```text
  abilityend|mon:Slaking,player-1,1|ability:Truant|from:ability:Neutralizing Gas|of:Weezing,player-2,1
  ```
* **Desired State in `battler-state`**:
  - `Slaking`'s volatile ability is cleared.
  - `Weezing`'s base ability is known as `Neutralizing Gas`.
* **Desired State in `battler-log-formatter`**:
  - Emits **2** notices:
    1. `{ type: "Ability", name: "Neutralizing Gas", mon: "The opposing Weezing's" }` (suppressing ability)
    2. `{ type: "Ability", name: "Truant", mon: "Slaking's" }` (suppressed ability)
* **Implementation Details**:
  - **`battler-state`**: `modify_state_from_effect` clears Slaking's volatile ability on `abilityend`. `record_effect_from_mon` records Weezing's base appearance ability from `from:ability` with `of`.
  - **`battler-log-formatter`**: The source effect rule (`hasSourceEffectType: "Ability"`) with `hasOf: true` (`sourceFirst`) emits the notice for `Weezing`, while the general ability rule (`targetFirst`) emits the ending ability notice for `Slaking`.

---

### Scenario 6: Self Item Reveal (`itemstart` without `source`)
* **Context**: Item announced on self (e.g. Air Balloon on switch-in, Choice lock).
* **Sample Log**:
  ```text
  itemstart|mon:Squirtle,player-1,1|item:Air Balloon
  ```
* **Desired State in `battler-state`**:
  - `Squirtle`'s item set to `Some("Air Balloon")`. `previous_item` remains `None`.
* **Desired State in `battler-log-formatter`**:
  - Emits 1 notice: `{ type: "Item", name: "Air Balloon", mon: "Squirtle's" }`.
* **Implementation Details**:
  - **`battler-state`**: `modify_state_from_effect` sets `Squirtle`'s held item to `Some("Air Balloon")`.
  - **`battler-log-formatter`**: Matched by rule `titleIn: ["item", "itemstart", "activate"]` with `monResolution: "targetFirst"`.

---

### Scenario 7: Item Theft / Transfer (`itemstart` with `source`)
* **Context**: Item stolen from victim and given to attacker (e.g. Thief, Covet). In standard battle execution, the engine emits `itemend` on the victim (clearing its item) immediately followed by `itemstart` on the thief:
  ```text
  itemend|mon:Tyranitar,player-2,1|item:Choice Band|from:move:Thief|of:Aipom,player-1,1
  itemstart|mon:Aipom,player-1,1|item:Choice Band|source:Tyranitar,player-2,1|from:move:Thief
  ```
* **Sample Log (`itemstart`)**:
  ```text
  itemstart|mon:Aipom,player-1,1|item:Choice Band|source:Tyranitar,player-2,1|from:move:Thief
  ```
* **Desired State in `battler-state`**:
  - Target (`Aipom`): Held item becomes `Some("Choice Band")`.
  - Source (`Tyranitar`): Held item cleared by the paired `itemend`; `previous_item` recorded as `Some("Choice Band")` (or learned as fallback by `itemstart` if previously unknown).
* **Desired State in `battler-log-formatter`**:
  - Emits **2** notices:
    1. `{ type: "Item", name: "Choice Band", mon: "The opposing Tyranitar's" }` (victim losing item)
    2. `{ type: "Item", name: "Choice Band", mon: "Aipom's" }` (thief gaining item)
* **Implementation Details**:
  - **`battler-state`**: Preceding `itemend` clears victim's held item and sets `previous_item`; `itemstart` sets target's item and ensures source's `previous_item` is recorded via `record_source_previous_item_if_unknown`.
  - **`battler-log-formatter`**: Rule `titleIn: ["itemstart", "itemend"], hasSource: true` emits `sourceOnly` notice; rule `titleIn: ["item", "itemstart", "activate"]` emits `targetFirst` notice.

---

### Scenario 8: Standard Item Consumption (`itemend` without `source`)
* **Context**: Mon consumes its own berry or consumable item (e.g. Sitrus Berry, Focus Sash, Booster Energy).
* **Sample Log**:
  ```text
  itemend|mon:Snorlax,player-1,1|item:Sitrus Berry|eat
  ```
* **Desired State in `battler-state`**:
  - `Snorlax`'s `item` cleared to `None`.
  - `Snorlax`'s `previous_item` updated to `Some("Sitrus Berry")`.
* **Desired State in `battler-log-formatter`**:
  - Emits 1 notice: `{ type: "Item", name: "Sitrus Berry", mon: "Snorlax's" }`.
* **Implementation Details**:
  - **`battler-state`**: `modify_state_from_effect` unifies target to `mon` (since `source` is absent), clears its held item, and records the item in `previous_item`.
  - **`battler-log-formatter`**: Matches rule `titleIn: ["itemend"], hasSource: false` for `targetFirst`.

---

### Scenario 9: Stolen Item Consumed on Field (`itemend` with `source`)
* **Context**: Attacker uses Bug Bite or Pluck to steal and instantly eat the opponent's berry.
* **Sample Log**:
  ```text
  itemend|mon:Scizor,player-1,1|item:Sitrus Berry|eat|source:Snorlax,player-2,1
  ```
* **Desired State in `battler-state`**:
  - **Source (`Snorlax`)**: The item belonged to Snorlax. Snorlax's `item` is cleared, and `previous_item` is updated to `Some("Sitrus Berry")`. Even if Snorlax previously consumed another item earlier in the battle, `previous_item` properly updates to this berry.
  - **Target (`Scizor`)**: Scizor only ate the stolen berry. Scizor's own held item (e.g. Life Orb) is **strictly preserved**. Scizor's `previous_item` remains untouched.
* **Desired State in `battler-log-formatter`**:
  - Emits **only 1** notice:
    - `{ type: "Item", name: "Sitrus Berry", mon: "The opposing Snorlax's" }`
  - Scizor receives **no** Item badge. The action text already states Scizor stole and ate Snorlax's berry, and follow-up healing/boosts credit Scizor.
* **Implementation Details**:
  - **`battler-state`** ([`log_handler.rs`](./battler-state/src/state/log_handler.rs)): Unifies the itemend target resolution:
    ```rust
    let target = entry.value::<MonName>("source").unwrap_or(mon);
    ```
    All item clearance and `previous_item` assignment operations apply solely to `source`.
  - **`battler-log-formatter`** ([`notice-rules.json`](./js-clients/battler-log-formatter/src/config/notice-rules.json)):
    - The generic `hasEffectType: "Item"` rule has `"titleNotIn": ["itemend"]`.
    - The `targetFirst` Item rule is separated so that `itemend` requires `"hasSource": false`.
    - The `sourceOnly` rule (`titleIn: ["itemstart", "itemend"], hasSource: true`) matches Snorlax.
    - Result: Only Snorlax receives the notice badge; no spurious badge is created for Scizor.

---

### Scenario 10: Secondary Item / Ability Triggers (`damage` / `heal` with `from` and `of`)
* **Context**: Life Orb recoil vs. Rocky Helmet / Rough Skin retaliation.
* **Sample Logs**:
  ```text
  damage|mon:Charizard,player-1,1|from:item:Life Orb|damage:10/100
  damage|mon:Lucario,player-1,1|from:ability:Rough Skin|of:Garchomp,player-2,1|damage:12/100
  ```
* **Desired State & Implementation**:
  - **Life Orb (no `of`)**: Implicitly belongs to `Charizard`. `battler-state` learns Charizard holds Life Orb; formatter emits `[Charizard's Life Orb]`.
  - **Rough Skin (with `of`)**: Attributed to `Garchomp`. `battler-state` learns Garchomp has Rough Skin; formatter emits `[The opposing Garchomp's Rough Skin]` alongside `[Lucario -12%]`.

---

### Scenario 11: Mirror Match Mon Disambiguation & Spectator View
* **Context**: Both sides have the same Pokémon species (e.g. Ralts vs. Ralts) viewed from a spectator or observer perspective.
* **Sample Log**:
  ```text
  abilitystart|mon:Ralts,player-1,1|ability:Insomnia|source:Ralts,player-2,1|from:ability:Trace
  ```
* **Desired State & Implementation**:
  - In mirror matches or spectator views where both combatants share the exact same formatted string label (e.g. both resolving to `"The opposing Ralts's"`), notice deduplication relies strictly on `areUiMonsEqual(n.monRef, monRef)`.
  - Identity is determined **exclusively by `monRef`** (`side`, `position`, `player`, and `name`), completely bypassing display strings.
  - As a result, both Pokémon receive their own distinct notice badges without being incorrectly collapsed.

---

## 4. Notice Rules Engine Configuration

The declarative rules in [`notice-rules.json`](./js-clients/battler-log-formatter/src/config/notice-rules.json) enforce this contract:

```json
[
  // 1. Source effect with 'of' (e.g. contact ability/item damage or contact replacement) -> attributes to 'of'
  {
    "condition": { "hasSourceEffectType": "Ability", "titleNotIn": ["abilitystart"] },
    "notice": { "type": "Ability", "nameFromPath": "source_effect.name", "monResolution": "sourceFirst" }
  },
  {
    "condition": { "hasSourceEffectType": "Ability", "titleIn": ["abilitystart"], "hasOf": true },
    "notice": { "type": "Ability", "nameFromPath": "source_effect.name", "monResolution": "sourceFirst" }
  },
  {
    "condition": { "hasSourceEffectType": "Item", "titleNotIn": ["itemstart", "itemend"] },
    "notice": { "type": "Item", "nameFromPath": "source_effect.name", "monResolution": "sourceFirst" }
  },
  {
    "condition": { "hasSourceEffectType": "Item", "titleIn": ["itemstart", "itemend"], "hasOf": true },
    "notice": { "type": "Item", "nameFromPath": "source_effect.name", "monResolution": "sourceFirst" }
  },

  // 2. Relational transfer / reveal with 'source' -> attributes to 'source'
  {
    "condition": { "titleIn": ["abilitystart"], "hasContext": "ABILITY", "hasSource": true },
    "notice": { "type": "Ability", "nameFromContext": "ABILITY", "monResolution": "sourceOnly" }
  },
  {
    "condition": { "titleIn": ["itemstart", "itemend"], "hasContext": "ITEM", "hasSource": true },
    "notice": { "type": "Item", "nameFromContext": "ITEM", "monResolution": "sourceOnly" }
  },

  // 3. Main ability / item reveal / start / end -> attributes to 'mon' (targetFirst)
  {
    "condition": { "hasEffectType": "Ability" },
    "notice": { "type": "Ability", "nameFromPath": "effect.name", "monResolution": "targetFirst" }
  },
  {
    "condition": { "titleIn": ["ability", "abilitystart", "abilityend", "activate"], "hasContext": "ABILITY" },
    "notice": { "type": "Ability", "nameFromContext": "ABILITY", "monResolution": "targetFirst" }
  },
  {
    "condition": { "hasEffectType": "Item", "titleNotIn": ["itemend"] },
    "notice": { "type": "Item", "nameFromPath": "effect.name", "monResolution": "targetFirst" }
  },
  {
    "condition": { "titleIn": ["item", "itemstart", "activate"], "hasContext": "ITEM" },
    "notice": { "type": "Item", "nameFromContext": "ITEM", "monResolution": "targetFirst" }
  },
  {
    "condition": { "titleIn": ["itemend"], "hasContext": "ITEM", "hasSource": false },
    "notice": { "type": "Item", "nameFromContext": "ITEM", "monResolution": "targetFirst" }
  }
]
```

---

## 5. Automated Verification Test Suite

Every rule and edge case documented here is protected by automated tests:

* **State Machine Tests** ([`battler-state/src/state/tests.rs`](./battler-state/src/state/tests.rs)):
  - `itemend_records_previous_item`
  - `itemend_with_source_records_source_previous_item`
  - `itemend_with_source_preserves_target_held_item`
  - `itemend_with_source_updates_source_existing_previous_item`
  - `itemstart_with_and_without_source`
  - `abilitystart_with_source_and_from_binds_from_to_target_and_ability_to_both`
  - `abilitystart_with_from_ability_and_of_records_of_base_ability`
  - `abilitystart_does_not_overwrite_source_known_ability`
  - `abilityend_with_from_ability_and_of_records_of_base_ability_and_clears_volatile`
  - `records_ability_and_source_ability_from_abilitystart`
  - `records_ability_from_abilitystart_without_source`
  - `from_without_of_defaults_to_target_and_never_source`
  - `from_item_does_not_overwrite_known_item_and_records_previous_if_empty`
* **Log Formatter Tests** ([`js-clients/battler-log-formatter/tests/formatter.test.ts`](./js-clients/battler-log-formatter/tests/formatter.test.ts)):
  - `should format abilitystart without source (only target gets notice)`
  - `should format abilitystart with source (both target and source get notice)`
  - `should format two ability notices simultaneously on abilitystart with from:ability (Lingering Aroma)`
  - `should format abilityend with only target notice`
  - `should format abilityend with from and of (target gets ability, of gets from)`
  - `should format itemstart without source (only target gets item notice)`
  - `should format itemstart with source (both target and source get item notice)`
  - `should format itemend without source (only target gets item notice)`
  - `should format itemend with source (e.g. Bug Bite / Pluck, only source whose item ended gets item notice)`
  - `should attribute from with no of implicitly to target mon (never source)`
  - `should attribute from with of to the of mon`
  - `should keep distinct notices for mons with identical formatted names in mirror matches`
