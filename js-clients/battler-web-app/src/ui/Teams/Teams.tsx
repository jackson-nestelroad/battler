import React, { useState } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import {
  saveTeam,
  deleteTeam,
  setDefaultTeam,
  moveTeamUp,
  moveTeamDown,
} from "../../store/teamsSlice";
import type { MonData } from "battler-types";
import JsonEditor from "../Common/JsonEditor";
import Tabs from "../Common/Tabs";
import { parseShowdown, exportToShowdown } from "team-converter";

import styles from "./Teams.module.scss";

const SAMPLE_TEAMS: Record<string, Partial<MonData>[]> = {
  "Kanto Starters": [
    {
      name: "Bulbasaur",
      species: "Bulbasaur",
      ability: "Overgrow",
      moves: ["Tackle", "Vine Whip", "Growl"],
      level: 50,
    },
    {
      name: "Charmander",
      species: "Charmander",
      ability: "Blaze",
      moves: ["Scratch", "Ember", "Growl"],
      level: 50,
    },
    {
      name: "Squirtle",
      species: "Squirtle",
      ability: "Torrent",
      moves: ["Tackle", "Water Gun", "Tail Whip"],
      level: 50,
    },
    {
      name: "Pikachu",
      species: "Pikachu",
      ability: "Static",
      moves: ["Thunder Shock", "Thunderbolt", "Quick Attack", "Growl"],
      level: 50,
    },
  ],
};

export default function Teams() {
  const dispatch = useAppDispatch();
  const teams = useAppSelector((state) => state.teams.teams);
  const defaultTeam = useAppSelector((state) => state.teams.defaultTeam);
  const teamOrder = useAppSelector((state) => state.teams.teamOrder);
  const teamNames =
    teamOrder.length > 0 ? teamOrder.filter((name) => teams[name]) : Object.keys(teams);

  const [activeTeamName, setActiveTeamName] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [jsonText, setJsonText] = useState("");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [editorFormat, setEditorFormat] = useState<"json" | "showdown">("json");

  const handleSelectTeam = (name: string) => {
    setActiveTeamName(name);
    setEditName(name);
    setJsonText(JSON.stringify(teams[name], null, 2));
    setEditorFormat("json");
    setErrorMsg(null);
    setSuccessMsg(null);
    setIsEditing(true);
  };

  const handleCreateNew = () => {
    setActiveTeamName(null);
    setEditName("New Team");
    setJsonText(
      JSON.stringify(
        [
          {
            name: "Pikachu",
            species: "Pikachu",
            ability: "Static",
            moves: ["Tackle"],
            level: 50,
          },
        ],
        null,
        2,
      ),
    );
    setEditorFormat("json");
    setErrorMsg(null);
    setSuccessMsg(null);
    setIsEditing(true);
  };

  const handleLoadSample = (sampleName: string) => {
    setActiveTeamName(null);
    setEditName(sampleName);
    setJsonText(JSON.stringify(SAMPLE_TEAMS[sampleName], null, 2));
    setEditorFormat("json");
    setErrorMsg(null);
    setSuccessMsg(null);
    setIsEditing(true);
  };

  const handleBack = () => {
    setIsEditing(false);
    setActiveTeamName(null);
    setEditName("");
    setJsonText("");
    setEditorFormat("json");
    setErrorMsg(null);
    setSuccessMsg(null);
  };

  const handleSwitchFormat = (newFormat: "json" | "showdown") => {
    if (editorFormat === newFormat) return;
    setErrorMsg(null);
    setSuccessMsg(null);

    if (newFormat === "showdown") {
      try {
        const parsed = JSON.parse(jsonText);
        if (!Array.isArray(parsed)) {
          throw new Error("JSON must be an array of Pokémon.");
        }
        for (const mon of parsed) {
          if (!mon || typeof mon !== "object" || !mon.species) {
            throw new Error("Each Pokémon must have a species defined.");
          }
        }
        const showdownText = exportToShowdown(parsed as MonData[]);
        setJsonText(showdownText);
        setEditorFormat("showdown");
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setErrorMsg(`Failed to convert to Showdown format: ${msg}`);
      }
    } else {
      try {
        const parsed = parseShowdown(jsonText);
        if (parsed.length === 0) {
          throw new Error("No Pokémon found in Showdown text.");
        }
        const jsonStr = JSON.stringify(parsed, null, 2);
        setJsonText(jsonStr);
        setEditorFormat("json");
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setErrorMsg(`Failed to convert to JSON: ${msg}`);
      }
    }
  };

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMsg(null);
    setSuccessMsg(null);

    const name = editName.trim();
    if (!name) {
      setErrorMsg("Team name cannot be empty.");
      return;
    }

    let parsed: MonData[];
    if (editorFormat === "json") {
      try {
        const jsonParsed = JSON.parse(jsonText);
        if (!Array.isArray(jsonParsed)) {
          setErrorMsg("Team data must be a JSON array of Pokémon objects.");
          return;
        }
        if (jsonParsed.length === 0) {
          setErrorMsg("Team must contain at least one Pokémon.");
          return;
        }
        // Schema validation
        for (let i = 0; i < jsonParsed.length; i++) {
          const mon = jsonParsed[i];
          if (!mon || typeof mon !== "object") {
            setErrorMsg(`Index ${i}: Pokémon data must be a JSON object.`);
            return;
          }
          if (typeof mon.species !== "string" || !mon.species.trim()) {
            setErrorMsg(`Index ${i}: Missing or invalid "species" (string required).`);
            return;
          }
          if (!Array.isArray(mon.moves) || mon.moves.some((m: unknown) => typeof m !== "string")) {
            setErrorMsg(`Index ${i}: Missing or invalid "moves" (array of strings required).`);
            return;
          }
          if (
            mon.level !== undefined &&
            (typeof mon.level !== "number" || mon.level < 1 || mon.level > 100)
          ) {
            setErrorMsg(`Index ${i}: "level" must be a number between 1 and 100.`);
            return;
          }
          if (mon.shiny !== undefined && typeof mon.shiny !== "boolean") {
            setErrorMsg(`Index ${i}: "shiny" must be a boolean.`);
            return;
          }
        }
        parsed = jsonParsed as MonData[];
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setErrorMsg(`Invalid JSON structure: ${msg}`);
        return;
      }
    } else {
      try {
        parsed = parseShowdown(jsonText);
        if (parsed.length === 0) {
          setErrorMsg("Showdown text must contain at least one Pokémon.");
          return;
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setErrorMsg(`Invalid Showdown structure: ${msg}`);
        return;
      }
    }

    // Save
    dispatch(saveTeam({ name, members: parsed }));
    setActiveTeamName(name);
    setSuccessMsg(`Team "${name}" successfully saved!`);
  };

  const handleDelete = () => {
    if (!activeTeamName) return;
    if (window.confirm(`Are you sure you want to delete "${activeTeamName}"?`)) {
      dispatch(deleteTeam(activeTeamName));
      handleBack();
    }
  };

  return (
    <div className="page-container">
      <div className="dashboard-header">
        <h1>Teams Editor</h1>
        <p>Define custom Pokémon teams in JSON format to use in matchmaking challenges.</p>
      </div>

      <div className={styles.layout}>
        {!isEditing ? (
          /* Directory list view page */
          <section className={`card ${styles.directoryView}`}>
            <div className="card-header">
              <h3>Your Teams</h3>
              <button className="btn btn-success" onClick={handleCreateNew}>
                + Create New
              </button>
            </div>
            {teamNames.length === 0 ? (
              <p className={styles.emptyText}>No teams configured</p>
            ) : (
              <div className={styles.teamsList}>
                {teamNames.map((name) => (
                  <div key={name} className={styles.teamListItemContainer}>
                    <button
                      type="button"
                      className={styles.teamListItem}
                      onClick={() => handleSelectTeam(name)}
                    >
                      <span className={styles.teamNameLabel}>
                        {name}
                        {defaultTeam === name && (
                          <span className={styles.defaultBadge}>Default</span>
                        )}
                      </span>
                      <span className={styles.teamSizeBadge}>{teams[name]?.length || 0} Mons</span>
                    </button>
                    <div className={styles.orderControls}>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          dispatch(moveTeamUp(name));
                        }}
                        className={styles.orderBtn}
                        title="Move Up"
                      >
                        ▲
                      </button>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          dispatch(moveTeamDown(name));
                        }}
                        className={styles.orderBtn}
                        title="Move Down"
                      >
                        ▼
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}

            <div className={styles.samplesHeader}>
              <h3>Load Sample Templates</h3>
            </div>
            <div className={styles.samplesList}>
              {Object.keys(SAMPLE_TEAMS).map((sampleName) => (
                <button
                  key={sampleName}
                  className={`btn btn-secondary ${styles.sampleItem}`}
                  onClick={() => handleLoadSample(sampleName)}
                >
                  {sampleName}
                </button>
              ))}
            </div>
          </section>
        ) : (
          /* Full Screen JSON Editor view page */
          <section className={`card ${styles.editorView}`}>
            <form onSubmit={handleSave} className={styles.editorForm}>
              <div className={styles.editorHeader}>
                <div className={styles.headerLeft}>
                  <button
                    type="button"
                    className={`btn btn-secondary ${styles.backBtn}`}
                    onClick={handleBack}
                  >
                    ← Back to Teams
                  </button>
                  <input
                    type="text"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    placeholder="Enter team name"
                    className={styles.teamNameInput}
                    required
                  />
                </div>
                <div className={styles.headerActions}>
                  {activeTeamName && (
                    <button type="button" className="btn btn-danger" onClick={handleDelete}>
                      Delete Team
                    </button>
                  )}
                  {activeTeamName && defaultTeam !== activeTeamName && (
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => {
                        dispatch(setDefaultTeam(activeTeamName));
                        setSuccessMsg(`"${activeTeamName}" successfully marked as default.`);
                      }}
                    >
                      Set as Default
                    </button>
                  )}
                  <button type="submit" className="btn btn-primary">
                    Save Team
                  </button>
                </div>
              </div>

              <Tabs
                options={[
                  { value: "json", label: "JSON" },
                  { value: "showdown", label: "Showdown" },
                ]}
                active={editorFormat}
                onChange={handleSwitchFormat}
              />

              <JsonEditor
                value={jsonText}
                onChange={setJsonText}
                placeholder={
                  editorFormat === "json"
                    ? `[\n  {\n    "name": "Pikachu",\n    "species": "Pikachu",\n    "moves": ["Tackle"]\n  }\n]`
                    : "Pikachu\nAbility: Static\nLevel: 50\n- Tackle"
                }
                error={errorMsg}
                success={successMsg}
                required
              />
            </form>
          </section>
        )}
      </div>
    </div>
  );
}
