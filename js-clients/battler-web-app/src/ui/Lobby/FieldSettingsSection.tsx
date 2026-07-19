import type { FieldEnvironment, TimeOfDay } from "battler-types";
import styles from "./ProposalForm.module.scss";

export interface FieldSettingsState {
  weather: string;
  terrain: string;
  environment: FieldEnvironment;
  timeOfDay: TimeOfDay;
}

interface FieldSettingsSectionProps {
  fieldSettings: FieldSettingsState;
  onChange: (fields: Partial<FieldSettingsState>) => void;
}

export default function FieldSettingsSection({
  fieldSettings,
  onChange,
}: FieldSettingsSectionProps) {
  return (
    <div className={styles.advancedSection}>
      <h4 className="mb-s">Field</h4>
      <div className="flex-row flex-mobile-col gap-m">
        <div className="form-group flex-1">
          <label htmlFor="weather">Default weather</label>
          <select
            id="weather"
            value={fieldSettings.weather}
            onChange={(e) => onChange({ weather: e.target.value })}
          >
            <option value="None">None</option>
            <option value="rainweather">Rain</option>
            <option value="harshsunlight">Harsh Sunlight</option>
            <option value="sandstormweather">Sandstorm</option>
            <option value="hailweather">Hail</option>
            <option value="snowweather">Snow</option>
            <option value="heavyrainweather">Heavy Rain</option>
            <option value="extremelyharshsunlight">Extremely Harsh Sunlight</option>
            <option value="strongwinds">Strong Winds</option>
          </select>
        </div>

        <div className="form-group flex-1">
          <label htmlFor="terrain">Default terrain</label>
          <select
            id="terrain"
            value={fieldSettings.terrain}
            onChange={(e) => onChange({ terrain: e.target.value })}
          >
            <option value="None">None</option>
            <option value="electricterrain">Electric Terrain</option>
            <option value="grassyterrain">Grassy Terrain</option>
            <option value="mistyterrain">Misty Terrain</option>
            <option value="psychicterrain">Psychic Terrain</option>
          </select>
        </div>

        <div className="form-group flex-1">
          <label htmlFor="environment">Environment</label>
          <select
            id="environment"
            value={fieldSettings.environment}
            onChange={(e) => onChange({ environment: e.target.value as FieldEnvironment })}
          >
            <option value="Normal">Normal</option>
            <option value="Cave">Cave</option>
            <option value="Sand">Sand</option>
            <option value="Water">Water</option>
            <option value="Ice">Ice</option>
            <option value="Sky">Sky</option>
            <option value="Grass">Grass</option>
            <option value="Volcano">Volcano</option>
          </select>
        </div>

        <div className="form-group flex-1">
          <label htmlFor="timeOfDay">Time of day</label>
          <select
            id="timeOfDay"
            value={fieldSettings.timeOfDay}
            onChange={(e) => onChange({ timeOfDay: e.target.value as TimeOfDay })}
          >
            <option value="Day">Day</option>
            <option value="Morning">Morning</option>
            <option value="Evening">Evening</option>
            <option value="Night">Night</option>
          </select>
        </div>
      </div>
    </div>
  );
}
