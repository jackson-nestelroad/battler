import { describe, expect, it } from 'vitest';
import { getLogPatterns } from '../src/pattern_reconstructor.js';
import type { UiLogEntry } from 'battler-state';

describe('Pattern Reconstructor', () => {
  it('should reconstruct basic stat boost', () => {
    const entry: UiLogEntry = {
      StatBoost: {
        mon: {} as any,
        stat: 'atk',
        by: 2n,
        effect: {
          additional: {}
        } as any
      }
    };
    expect(getLogPatterns(entry)[0]).toBe('boost');
  });

  it('should reconstruct ability stat drop', () => {
    const entry: UiLogEntry = {
      StatBoost: {
        mon: {} as any,
        stat: 'atk',
        by: -1n,
        effect: {
          source_effect: { effect_type: 'Ability', name: 'Intimidate' },
          target: {} as any,
          additional: {}
        } as any
      }
    };
    expect(getLogPatterns(entry)[0]).toBe('unboost|from:ability:Intimidate');
  });
});
