import { useState } from "react";
import styles from "./ProposalForm.module.scss";

export interface CustomRulesState {
  preset: "none" | "standard" | "standarddoubles" | "flatrules" | "custom";
  speciesClause: boolean;
  sleepClause: boolean;
  itemClause: boolean;
  nicknameClause: boolean;
  ohkoClause: boolean;
  evasionClause: boolean;
  endlessBattleClause: boolean;
  megaEvolution: boolean;
  zMoves: boolean;
  dynamax: boolean;
  terastallization: boolean;
  bagItems: boolean;
  pickedTeamSizeAuto: boolean;
  pickedTeamSize: number;
  adjustLevelDownEnabled: boolean;
  adjustLevelDown: number;
  defaultLevel: number;
  maxLevel: number;
  rulesList: string[];
}

interface AdvancedRulesSectionProps {
  customRules: CustomRulesState;
  onChange: (fields: Partial<CustomRulesState>) => void;
}

export default function AdvancedRulesSection({ customRules, onChange }: AdvancedRulesSectionProps) {
  // Rules builder local form state
  const [ruleAction, setRuleAction] = useState<"clause" | "ban" | "allow" | "repeal">("clause");
  const [ruleCategory, setRuleCategory] = useState<string>("");
  const [ruleValue, setRuleValue] = useState<string>("");

  const handleAddRule = () => {
    const trimmed = ruleValue.trim();
    if (!trimmed) return;
    let formatted = "";
    if (ruleAction === "clause") {
      formatted = trimmed;
    } else if (ruleAction === "repeal") {
      formatted = `! ${trimmed}`;
    } else {
      const prefix = ruleAction === "ban" ? "-" : "+";
      formatted = ruleCategory
        ? `${prefix} ${ruleCategory}: ${trimmed}`
        : `${prefix} ${trimmed}`;
    }
    if (formatted && !customRules.rulesList.includes(formatted)) {
      onChange({
        rulesList: [...customRules.rulesList, formatted],
      });
    }
    setRuleValue("");
  };

  const handleRemoveRule = (index: number) => {
    onChange({
      rulesList: customRules.rulesList.filter((_, idx) => idx !== index),
    });
  };

  const isCustom = customRules.preset === "custom";

  return (
    <div className={styles.advancedSection}>
      <h4 className="mb-s">Format rules</h4>
      <div className="flex-row flex-mobile-col gap-m align-end">
        <div className="form-group flex-1">
          <label htmlFor="rulesetPreset">Ruleset preset</label>
          <select
            id="rulesetPreset"
            value={customRules.preset}
            onChange={(e) => onChange({ preset: e.target.value as CustomRulesState["preset"] })}
          >
            <option value="none">None</option>
            <option value="standard">Standard</option>
            <option value="standarddoubles">Standard Doubles</option>
            <option value="flatrules">Flat Rules</option>
            <option value="custom">Custom</option>
          </select>
        </div>
      </div>

      {isCustom && (
        <div className="flex-col gap-m mt-m">
          <div className={styles.checkboxGrid}>
            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.speciesClause}
                onChange={(e) => onChange({ speciesClause: e.target.checked })}
              />
              <span>Species Clause</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.sleepClause}
                onChange={(e) => onChange({ sleepClause: e.target.checked })}
              />
              <span>Sleep Clause</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.itemClause}
                onChange={(e) => onChange({ itemClause: e.target.checked })}
              />
              <span>Item Clause</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.nicknameClause}
                onChange={(e) => onChange({ nicknameClause: e.target.checked })}
              />
              <span>Nickname Clause</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.ohkoClause}
                onChange={(e) => onChange({ ohkoClause: e.target.checked })}
              />
              <span>OHKO Clause</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.evasionClause}
                onChange={(e) => onChange({ evasionClause: e.target.checked })}
              />
              <span>Evasion Clause</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.endlessBattleClause}
                onChange={(e) => onChange({ endlessBattleClause: e.target.checked })}
              />
              <span>Endless Battle Clause</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.megaEvolution}
                onChange={(e) => onChange({ megaEvolution: e.target.checked })}
              />
              <span>Mega Evolution</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.zMoves}
                onChange={(e) => onChange({ zMoves: e.target.checked })}
              />
              <span>Z-Moves</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.dynamax}
                onChange={(e) => onChange({ dynamax: e.target.checked })}
              />
              <span>Dynamax</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.terastallization}
                onChange={(e) => onChange({ terastallization: e.target.checked })}
              />
              <span>Terastallization</span>
            </label>

            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={customRules.bagItems}
                onChange={(e) => onChange({ bagItems: e.target.checked })}
              />
              <span>Bag items</span>
            </label>
          </div>

          <div className="flex-row flex-mobile-col gap-m mt-s">
            <div className="form-group flex-1">
              <label htmlFor="customPickedTeamSize">Picked team size</label>
              <input
                id="customPickedTeamSize"
                type="number"
                min="1"
                max="6"
                value={customRules.pickedTeamSizeAuto ? "" : customRules.pickedTeamSize}
                onChange={(e) => onChange({ pickedTeamSize: Number(e.target.value) })}
                placeholder={customRules.pickedTeamSizeAuto ? "Auto" : undefined}
                disabled={customRules.pickedTeamSizeAuto}
              />
              <label className={styles.checkboxLabel} style={{ marginTop: "var(--spacing-xs)" }}>
                <input
                  type="checkbox"
                  checked={customRules.pickedTeamSizeAuto}
                  onChange={(e) => onChange({ pickedTeamSizeAuto: e.target.checked })}
                />
                <span>Auto</span>
              </label>
            </div>

            <div className="form-group flex-1">
              <label htmlFor="customAdjustLevelDown">Adjust level down</label>
              <input
                id="customAdjustLevelDown"
                type="number"
                min="1"
                max="100"
                value={customRules.adjustLevelDownEnabled ? customRules.adjustLevelDown : ""}
                onChange={(e) => onChange({ adjustLevelDown: Number(e.target.value) })}
                placeholder={customRules.adjustLevelDownEnabled ? undefined : "None"}
                disabled={!customRules.adjustLevelDownEnabled}
              />
              <label className={styles.checkboxLabel} style={{ marginTop: "var(--spacing-xs)" }}>
                <input
                  type="checkbox"
                  checked={customRules.adjustLevelDownEnabled}
                  onChange={(e) => onChange({ adjustLevelDownEnabled: e.target.checked })}
                />
                <span>Enable custom limit</span>
              </label>
            </div>

            <div className="form-group flex-1">
              <label htmlFor="customDefaultLevel">Default level</label>
              <input
                id="customDefaultLevel"
                type="number"
                min="1"
                max="100"
                value={customRules.defaultLevel}
                onChange={(e) => onChange({ defaultLevel: Number(e.target.value) })}
              />
            </div>

            <div className="form-group flex-1">
              <label htmlFor="customMaxLevel">Max level</label>
              <input
                id="customMaxLevel"
                type="number"
                min="1"
                max="100"
                value={customRules.maxLevel}
                onChange={(e) => onChange({ maxLevel: Number(e.target.value) })}
              />
            </div>
          </div>

          {/* Active Rules List */}
          <div className="flex-col gap-s w-full border-top pt-m mt-m">
            <span className={styles.sideHeaderLabel} style={{ fontSize: "var(--font-size-s)" }}>
              Other rules
            </span>
            <div className="flex-row flex-wrap gap-xs mt-xs">
              {customRules.rulesList.map((rule, idx) => (
                <div key={idx} className={styles.ruleBadge}>
                  <span>{rule}</span>
                  <button
                    type="button"
                    className={styles.removeRuleBtn}
                    onClick={() => handleRemoveRule(idx)}
                  >
                    &times;
                  </button>
                </div>
              ))}
              {customRules.rulesList.length === 0 && (
                <span className="text-secondary italic">None</span>
              )}
            </div>
          </div>

          {/* Rules Builder Form */}
          <div className={styles.rulesBuilderRow}>
            <div className={`form-group ${styles.actionField}`}>
              <label htmlFor="ruleAction">Action</label>
              <select
                id="ruleAction"
                value={ruleAction}
                onChange={(e) => setRuleAction(e.target.value as typeof ruleAction)}
              >
                <option value="clause">Clause</option>
                <option value="ban">Ban (-)</option>
                <option value="allow">Allow (+)</option>
                <option value="repeal">Repeal (!)</option>
              </select>
            </div>

            {(ruleAction === "ban" || ruleAction === "allow") && (
              <div className={`form-group ${styles.categoryField}`}>
                <label htmlFor="ruleCategory">Category</label>
                <select
                  id="ruleCategory"
                  value={ruleCategory}
                  onChange={(e) => setRuleCategory(e.target.value)}
                >
                  <option value="">None</option>
                  <option value="Move Tag">Move tag</option>
                  <option value="Item Tag">Item tag</option>
                  <option value="Ability Tag">Ability tag</option>
                </select>
              </div>
            )}

            <div className={`form-group ${styles.valueField}`}>
              <label htmlFor="ruleValue">Value</label>
              <input
                id="ruleValue"
                type="text"
                placeholder={
                  ruleAction === "clause"
                    ? "e.g., Same Type Clause"
                    : ruleAction === "repeal"
                      ? "e.g., Sleep Clause"
                      : "e.g., Thunderbolt or Pikachu"
                }
                value={ruleValue}
                onChange={(e) => setRuleValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleAddRule();
                  }
                }}
              />
            </div>

            <button
              type="button"
              className={`btn btn-secondary ${styles.addBtnField}`}
              onClick={handleAddRule}
              disabled={!ruleValue.trim()}
            >
              + Add rule
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
