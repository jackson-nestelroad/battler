import { serializeChoice } from "battler-choice-wasm";
import { Choice } from "battler-types";

export class ChoiceBuilder {
  private constructor(private value: Choice) {}

  static pass(): ChoiceBuilder {
    return new ChoiceBuilder("pass");
  }

  static random(): ChoiceBuilder {
    return new ChoiceBuilder("random");
  }

  static randomAll(): ChoiceBuilder {
    return new ChoiceBuilder("randomall");
  }

  static escape(): ChoiceBuilder {
    return new ChoiceBuilder("escape");
  }

  static forfeit(): ChoiceBuilder {
    return new ChoiceBuilder("forfeit");
  }

  static shift(): ChoiceBuilder {
    return new ChoiceBuilder("shift");
  }

  static team(mons: number[]): ChoiceBuilder {
    return new ChoiceBuilder({ team: { mons } });
  }

  static switch(mon: number | null = null): ChoiceBuilder {
    return new ChoiceBuilder({ switch: { mon } });
  }

  static learnMove(forgetMoveSlot: number): ChoiceBuilder {
    return new ChoiceBuilder({ learnmove: { forget_move_slot: forgetMoveSlot } });
  }

  static select(mon: number | null = null): ChoiceBuilder {
    return new ChoiceBuilder({ select: { mon } });
  }

  static move(slot: number): MoveChoiceBuilder {
    return new MoveChoiceBuilder(slot);
  }

  static item(item: string): ItemChoiceBuilder {
    return new ItemChoiceBuilder(item);
  }

  build(): Choice {
    return this.value;
  }

  toString(): string {
    return serializeChoice(this.value);
  }
}

export class MoveChoiceBuilder {
  private _slot: number;
  private _target: number | null = null;
  private _mega = false;
  private _zMove = false;
  private _ultra = false;
  private _dyna = false;
  private _tera = false;
  private _randomTarget = false;

  constructor(slot: number) {
    this._slot = slot;
  }

  target(position: number): this {
    this._target = position;
    return this;
  }

  mega(enabled = true): this {
    this._mega = enabled;
    return this;
  }

  zMove(enabled = true): this {
    this._zMove = enabled;
    return this;
  }

  ultra(enabled = true): this {
    this._ultra = enabled;
    return this;
  }

  dyna(enabled = true): this {
    this._dyna = enabled;
    return this;
  }

  tera(enabled = true): this {
    this._tera = enabled;
    return this;
  }

  randomTarget(enabled = true): this {
    this._randomTarget = enabled;
    return this;
  }

  build(): Choice {
    return {
      move: {
        slot: this._slot,
        target: this._target,
        mega: this._mega,
        z_move: this._zMove,
        ultra: this._ultra,
        dyna: this._dyna,
        tera: this._tera,
        random_target: this._randomTarget,
      },
    };
  }

  toString(): string {
    return serializeChoice(this.build());
  }
}

export class ItemChoiceBuilder {
  private _item: string;
  private _target: number | null = null;
  private _additionalInput: string[] = [];

  constructor(item: string) {
    this._item = item;
  }

  target(position: number): this {
    this._target = position;
    return this;
  }

  additionalInput(input: string[]): this {
    this._additionalInput = input;
    return this;
  }

  build(): Choice {
    return {
      item: {
        item: this._item,
        target: this._target,
        additional_input: this._additionalInput,
      },
    };
  }

  toString(): string {
    return serializeChoice(this.build());
  }
}
