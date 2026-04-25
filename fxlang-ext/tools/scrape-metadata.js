const fs = require("fs");
const path = require("path");

const REPO_ROOT = path.join(__dirname, "..", "..");
const VARIABLE_RS = path.join(
  REPO_ROOT,
  "battler",
  "src",
  "effect",
  "fxlang",
  "variable.rs",
);
const FUNCTIONS_RS = path.join(
  REPO_ROOT,
  "battler",
  "src",
  "effect",
  "fxlang",
  "functions.rs",
);
const EFFECT_RS = path.join(
  REPO_ROOT,
  "battler",
  "src",
  "effect",
  "fxlang",
  "effect.rs",
);
const EVAL_RS = path.join(
  REPO_ROOT,
  "battler",
  "src",
  "effect",
  "fxlang",
  "eval.rs",
);
const OUTPUT_FILE = path.join(__dirname, "..", "metadata.json");

function scrapeTypeMappings(filePath) {
  const content = fs.readFileSync(filePath, "utf8");
  const lines = content.split("\n");
  const mapping = {};

  let insideValueType = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line.includes("pub fn value_type(&self) -> ValueType {")) {
      insideValueType = true;
      continue;
    }

    if (insideValueType) {
      if (line === "}") {
        insideValueType = false;
        continue;
      }

      // Match: Self::Variant(_) => ValueType::Type,
      const match = line.match(
        /Self::(\w+)(?:\(.*\))?\s*=>\s*ValueType::(\w+)/,
      );
      if (match) {
        const variant = match[1];
        const type = match[2];
        mapping[variant] = type;
      }
    }
  }
  return mapping;
}

function scrapeVariables(filePath, typeMapping) {
  const content = fs.readFileSync(filePath, "utf8");
  const lines = content.split("\n");

  const metadata = {
    global: {},
    types: {},
  };

  let currentType = "global";
  let docBuffer = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    // Check for doc comments
    if (line.startsWith("///")) {
      docBuffer.push(line.replace("///", "").trim());
      continue;
    }

    // Check for type transitions
    // Handles: value.mon_handle(), ValueRef::Mon, ValueRefMut::Mon, etc.
    // We look for transitions followed by { which usually indicates a match arm or if block
    const typeMatch = line.match(
      /(?:value\.(\w+)_handle\(\)|ValueRef(?:Mut)?::(\w+)).*?(?:\{|=>\s*\{)/,
    );
    if (typeMatch) {
      let typeName = typeMatch[1] || typeMatch[2];
      // Normalize type name (e.g., mon -> Mon, active_move -> ActiveMove)
      currentType = typeName
        .split("_")
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join("");
      if (currentType === "Effect" && line.includes("ActiveMove"))
        currentType = "ActiveMove";

      if (!metadata.types[currentType]) metadata.types[currentType] = {};
      docBuffer = [];
      continue;
    }

    // Check for member match arms
    // Matches: "id" => ...
    const memberMatch = line.match(/^"([a-z0-9_]+)"\s*=>/);
    if (memberMatch) {
      const memberName = memberMatch[1];
      let returnType = "Undefined";

      // Look ahead for the type (ValueRef::Type, ValueRefMut::Type, or Value::Type)
      // We search up to 10 lines ahead for a type indicator
      for (let j = i; j < Math.min(i + 10, lines.length); j++) {
        const nextLine = lines[j].trim();
        const typeIndicator = nextLine.match(
          /(?:ValueRef(?:Mut)?|Value)::(\w+)/,
        );
        if (typeIndicator) {
          returnType = typeMapping[typeIndicator[1]] || typeIndicator[1];
          break;
        }
        // If we hit the next member or end of block, stop
        if (j > i && (nextLine.match(/^"[a-z0-9_]+"/) || nextLine === "}"))
          break;
      }

      const description = docBuffer.join(" ");
      const memberData = { description, type: returnType };

      if (currentType === "global") {
        metadata.global[memberName] = memberData;
      } else {
        // If already exists, don't overwrite if existing has a real type and new one is Undefined
        if (
          metadata.types[currentType][memberName] &&
          metadata.types[currentType][memberName].type !== "Undefined" &&
          returnType === "Undefined"
        ) {
          // Skip
        } else {
          metadata.types[currentType][memberName] = memberData;
        }
      }
      docBuffer = [];
    } else if (line !== "" && !line.startsWith("//") && !line.startsWith("}")) {
      // No reset here
    }
  }

  return metadata;
}

function scrapeBuiltInVariables(filePath) {
  // These are the truly global variables injected into every FxLang context
  // Conditional variables (like target, source, etc.) are handled per-event in scrapeEvents.
  const globalVars = {
    this: { type: "Effect", optional: false },
    battle: { type: "Battle", optional: false },
    field: { type: "Field", optional: false },
    format: { type: "Format", optional: false },
    effect_state: { type: "EffectState", optional: true },
    effect_target: { type: "Mon", optional: true },
    event_origin: { type: "Mon", optional: true },
  };
  return globalVars;
}

function scrapeFunctions(filePath) {
  const content = fs.readFileSync(filePath, "utf8");
  const lines = content.split("\n");

  const functions = {};
  const funcMap = {}; // Maps external name to internal fn name

  let insideMatch = false;

  // Phase 1: Map external names to internal function names
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line.includes("match function_name {")) {
      insideMatch = true;
      continue;
    }

    if (insideMatch && (line.startsWith("_ =>") || line.startsWith("}"))) {
      if (line.startsWith("_ =>")) {
        insideMatch = false;
        break;
      }
      continue;
    }

    if (!insideMatch) continue;

    const match = line.match(/^"([a-z0-9_]+)"\s*=>\s*([a-z0-9_]+)?/);
    if (match) {
      const extName = match[1];
      let intName = match[2];

      if (!intName || intName === "{") {
        for (let j = i + 1; j < lines.length; j++) {
          const nextLine = lines[j].trim();
          const nextMatch = nextLine.match(/([a-z0-9_]+)\(/);
          if (nextMatch) {
            intName = nextMatch[1];
            if (intName === "map" || intName === "Ok" || intName === "Some")
              continue;
            break;
          }
          if (nextLine.startsWith('"') || nextLine.startsWith("_ =>")) break;
        }
      }

      if (intName) {
        funcMap[extName] = intName;
      }
    }
  }

  // Phase 2: Find function definitions and extract doc comments + @returns
  const fnContents = content.split("\n");
  for (const [extName, intName] of Object.entries(funcMap)) {
    const defRegex = new RegExp(`fn\\s+${intName}\\s*\\(`);
    let defLine = -1;
    for (let i = 0; i < fnContents.length; i++) {
      if (fnContents[i].trim().match(defRegex)) {
        defLine = i;
        break;
      }
    }

    if (defLine !== -1) {
      let docBuffer = [];
      let returnType = "Undefined";
      let parameters = [];
      let flags = [];
      for (let i = defLine - 1; i >= 0; i--) {
        const line = fnContents[i].trim();
        if (line.startsWith("///")) {
          const docLine = line.replace("///", "").trim();

          const retMatch = docLine.match(/@returns\s*\{(.*)\}/);
          const paramMatch = docLine.match(
            /@param\s*\{(.*)\}\s*(?:\[(\w+)\]|(\w+))\s*(.*)/,
          );
          const flagMatch = docLine.match(/@flag\s*(\w+)\s*(.*)/);

          if (retMatch) {
            const rawType = retMatch[1];
            returnType = rawType.replace(/\[`ValueType::(\w+)`\]/g, "$1");
          } else if (paramMatch) {
            const rawType = paramMatch[1];
            const optional = !!paramMatch[2];
            const name = paramMatch[2] || paramMatch[3];
            const description = paramMatch[4];
            const type = rawType.replace(/\[`ValueType::(\w+)`\]/g, "$1");

            parameters.unshift({
              name,
              type,
              description,
              optional,
            });
          } else if (flagMatch) {
            const name = flagMatch[1];
            const description = flagMatch[2];
            flags.unshift({
              name,
              description,
            });
          } else {
            docBuffer.unshift(docLine);
          }
        } else if (
          line === "" ||
          line.startsWith("#[") ||
          line.startsWith("pub ")
        ) {
          continue;
        } else {
          break;
        }
      }
      functions[extName] = {
        description: docBuffer.join(" ").trim(),
        parameters,
        flags,
        type: returnType,
      };
    } else {
      functions[extName] = {
        description: "",
        parameters: [],
        flags: [],
        type: "Undefined",
      };
    }
  }

  return functions;
}

function scrapeEvents(effectFilePath, evalFilePath) {
  const effectContent = fs.readFileSync(effectFilePath, "utf8");
  const evalContent = fs.readFileSync(evalFilePath, "utf8");

  // 1. Parse CallbackFlags mapping
  const flagsMap = {};
  const flagRegex = /pub const (\w+):\s*u32\s*=\s*1\s*<<\s*(\d+);/g;
  let match;
  while ((match = flagRegex.exec(effectContent)) !== null) {
    flagsMap[match[1]] = 1 << parseInt(match[2], 10);
  }

  // 2. Parse CommonCallbackType bitmasks
  const commonTypesMap = {};
  const enumMatch = effectContent.match(/enum CommonCallbackType\s*{([^}]*)}/);
  if (enumMatch) {
    const body = enumMatch[1];
    const assignments = body
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    for (const assignment of assignments) {
      const parts = assignment.split("=");
      if (parts.length === 2) {
        const name = parts[0].trim();
        const expr = parts[1].replace(/CallbackFlag::/g, "").trim();
        const components = expr.split("|").map((s) => s.trim());
        let val = 0;
        for (const comp of components) {
          if (flagsMap[comp]) {
            val |= flagsMap[comp];
          }
        }
        commonTypesMap[name] = val;
      }
    }
  }

  // 3. Map CallbackFlags to Variables from eval.rs
  // We statically map what initialize_vars does based on its code
  const flagVars = {};
  if (flagsMap["TakesGeneralMon"])
    flagVars[flagsMap["TakesGeneralMon"]] = {
      mon: { type: "Mon", optional: false },
    };
  if (flagsMap["TakesTargetMon"])
    flagVars[flagsMap["TakesTargetMon"]] = {
      target: { type: "Mon", optional: true },
    };
  if (flagsMap["TakesSourceMon"])
    flagVars[flagsMap["TakesSourceMon"]] = {
      source: { type: "Mon", optional: true },
    };
  if (flagsMap["TakesUserMon"])
    flagVars[flagsMap["TakesUserMon"]] = {
      user: { type: "Mon", optional: false },
    };
  if (flagsMap["TakesSourceTargetMon"])
    flagVars[flagsMap["TakesSourceTargetMon"]] = {
      target: { type: "Mon", optional: true },
    };
  if (flagsMap["TakesEffect"])
    flagVars[flagsMap["TakesEffect"]] = {
      effect: { type: "Effect", optional: false },
    };
  if (flagsMap["TakesSourceEffect"])
    flagVars[flagsMap["TakesSourceEffect"]] = {
      source_effect: { type: "Effect", optional: false },
    };
  if (flagsMap["TakesActiveMove"])
    flagVars[flagsMap["TakesActiveMove"]] = {
      move: { type: "ActiveMove", optional: false },
    };
  if (flagsMap["TakesOptionalEffect"])
    flagVars[flagsMap["TakesOptionalEffect"]] = {
      effect: { type: "Effect", optional: true },
    };
  if (flagsMap["TakesSide"])
    flagVars[flagsMap["TakesSide"]] = {
      side: { type: "Side", optional: false },
    };
  if (flagsMap["TakesPlayer"])
    flagVars[flagsMap["TakesPlayer"]] = {
      player: { type: "Player", optional: false },
    };

  // 4. Parse BattleEvent descriptions
  const events = {};
  const lines = effectContent.split("\n");
  let docBuffer = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (line.startsWith("///")) {
      docBuffer.push(line.replace("///", "").trim());
    } else if (line.startsWith('#[string = "')) {
      const stringMatch = line.match(/#\[string = "(\w+)"\]/);
      if (stringMatch) {
        const eventName = stringMatch[1];
        const snakeEventName = eventName
          .replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`)
          .replace(/^_/, "");
        events[eventName] = {
          snakeName: snakeEventName,
          description: docBuffer.join("\n").trim(),
          variables: {},
        };
      }
      docBuffer = [];
    } else if (line === "" || line.startsWith("#[")) {
      // skip
    } else {
      docBuffer = [];
    }
  }

  // 5. Map BattleEvent to CommonCallbackType and populate variables
  const validEvents = {};
  const ctfMatch = effectContent.match(
    /pub fn callback_type_flags\(&self\)\s*->\s*u32\s*{([\s\S]*?)^    }/m,
  );
  if (ctfMatch) {
    const ctfBody = ctfMatch[1];
    const armRegex =
      /Self::(\w+)\s*=>\s*CommonCallbackType::(\w+)\s*as\s*u32,/g;
    let armMatch;
    while ((armMatch = armRegex.exec(ctfBody)) !== null) {
      const eventName = armMatch[1];
      const commonType = armMatch[2];

      if (events[eventName] && commonTypesMap[commonType]) {
        const bitmask = commonTypesMap[commonType];

        // Add variables for each set flag
        for (const [flagBit, varsMap] of Object.entries(flagVars)) {
          if ((bitmask & parseInt(flagBit, 10)) !== 0) {
            for (const [vName, vType] of Object.entries(varsMap)) {
              events[eventName].variables[vName] = vType;
            }
          }
        }

        const snakeName = events[eventName].snakeName;
        validEvents[snakeName] = {
          description: events[eventName].description,
          variables: events[eventName].variables,
        };
      }
    }
  }

  return validEvents;
}

function scrapeCommonFlags(filePath) {
  const content = fs.readFileSync(filePath, "utf8");
  const flags = new Set();
  const flagRegex = /self\.has_flag\("(\w+)"\)/g;

  let match;
  while ((match = flagRegex.exec(content)) !== null) {
    flags.add(match[1]);
  }

  return Array.from(flags).sort();
}

function scrapeEffectStateMembers(filePath) {
  const content = fs.readFileSync(filePath, "utf8");
  const lines = content.split("\n");
  const members = {};

  let docBuffer = [];
  let lastType = "Undefined";

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line.startsWith("///")) {
      const typeMatch = line.match(/\[`ValueType::(\w+)`\]/);
      if (typeMatch) {
        lastType = typeMatch[1];
      } else {
        docBuffer.push(line.replace("///", "").trim());
      }
      continue;
    }

    const constMatch = line.match(
      /const\s+\w+:\s*&'static\s+str\s*=\s*"([^"]+)"/,
    );
    if (constMatch) {
      const memberName = constMatch[1];
      members[memberName] = {
        description: docBuffer.join(" ").trim(),
        type: lastType,
      };
      docBuffer = [];
      lastType = "Undefined";
    } else {
      docBuffer = [];
    }
  }

  return members;
}

function main() {
  console.log("Scraping type mappings from value.rs...");
  const VALUE_RS = path.join(
    REPO_ROOT,
    "battler",
    "src",
    "effect",
    "fxlang",
    "value.rs",
  );
  const typeMappings = scrapeTypeMappings(VALUE_RS);

  console.log("Scraping fxlang metadata...");

  let existingMetadata = {};
  if (fs.existsSync(OUTPUT_FILE)) {
    existingMetadata = JSON.parse(fs.readFileSync(OUTPUT_FILE, "utf8"));
  }

  const vars = scrapeVariables(VARIABLE_RS, typeMappings);
  const builtInVars = scrapeBuiltInVariables(EVAL_RS);
  const funcs = scrapeFunctions(FUNCTIONS_RS);
  const commonFlags = scrapeCommonFlags(FUNCTIONS_RS);
  const events = scrapeEvents(EFFECT_RS, EVAL_RS);
  const effectStateMembers = scrapeEffectStateMembers(
    path.join(
      REPO_ROOT,
      "battler",
      "src",
      "effect",
      "fxlang",
      "effect_state.rs",
    ),
  );
  vars.types["EffectState"] = effectStateMembers;
  const fullMetadata = {
    variables: builtInVars,
    variable_members: vars.global,
    type_members: vars.types,
    functions: funcs,
    common_flags: commonFlags,
    events:
      Object.keys(events).length > 0 ? events : existingMetadata.events || {},
  };

  fs.writeFileSync(OUTPUT_FILE, JSON.stringify(fullMetadata, null, 2));
  console.log(`Metadata written to ${OUTPUT_FILE}`);

  // Update TextMate grammar with common flags
  const grammarPath = path.join(
    REPO_ROOT,
    "fxlang-ext",
    "syntaxes",
    "fxlang.tmLanguage.json",
  );
  if (fs.existsSync(grammarPath)) {
    const grammar = JSON.parse(fs.readFileSync(grammarPath, "utf8"));
    const builtin = ["true", "false", "undefined", "stop"];
    const allConstants = [...builtin, ...commonFlags];
    grammar.repository.constants.patterns[0].match =
      "\\b(" + allConstants.join("|") + ")\\b";
    fs.writeFileSync(grammarPath, JSON.stringify(grammar, null, 2) + "\n");
    console.log(`Updated grammar constants in ${grammarPath}`);
  }
}

main();
