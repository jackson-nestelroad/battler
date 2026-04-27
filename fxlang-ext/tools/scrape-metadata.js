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

function extractReturnTypes(lines, startIndex, typeMapping) {
  const returnTypes = new Set();
  let onlyApplicableToMove = false;
  
  for (let j = startIndex; j < Math.min(startIndex + 30, lines.length); j++) {
    const nextLine = lines[j].trim();
    
    if (nextLine.includes('.move_effect()')) {
      onlyApplicableToMove = true;
    }
    
    if (j > startIndex && (nextLine.match(/^"[a-z0-9_]+"(?:\s*\|\s*"[a-z0-9_]+")*\s*=>/) || nextLine.match(/(?:value\.(\w+)_handle\(\)|ValueRef(?:Mut)?::(\w+)).*?(?:\{|=>\s*\{)/))) {
      break;
    }
    
    const matches = nextLine.matchAll(/\b(?:ValueRef(?:Mut)?|Value)::(\w+)/g);
    for (const match of matches) {
      const type = typeMapping[match[1]] || match[1];
      returnTypes.add(type);
    }
  }
  return { returnTypes, onlyApplicableToMove };
}

function parseCommonCallbackTypeBitmasks(effectContent, flagsMap) {
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
  return commonTypesMap;
}

function parseBattleEventDescriptions(effectContent) {
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
  return events;
}

function parseInputVars(effectContent) {
  const inputVarsMap = {};
  const ivMatch = effectContent.match(/pub fn input_vars\(&self\)\s*->\s*&\[\(&str,\s*ValueType,\s*bool\)\]\s*{([\s\S]*?)^    }/m);
  if (ivMatch) {
    const ivBody = ivMatch[1];
    const ivLines = ivBody.split('\n');
    let currentEvents = [];
    let insideVars = false;
    
    for (let line of ivLines) {
      line = line.trim();
      if (line.includes('=>')) {
        const arrowIndex = line.indexOf('=>');
        const leftSide = line.substring(0, arrowIndex).trim();
        const rightSide = line.substring(arrowIndex + 2).trim();
        
        currentEvents = [];
        const eventMatches = [...leftSide.matchAll(/Self::(\w+)/g)];
        for (const em of eventMatches) {
          currentEvents.push(em[1]);
        }
        
        insideVars = true;
        
        const varMatches = [...rightSide.matchAll(/\("(\w+)",\s*ValueType::(\w+),\s*(\w+)\)/g)];
        for (const vm of varMatches) {
          for (const ev of currentEvents) {
            if (!inputVarsMap[ev]) inputVarsMap[ev] = [];
            inputVarsMap[ev].push({ name: vm[1], type: vm[2], optional: vm[3] === 'false' });
          }
        }
        
        if (rightSide.includes(']')) {
          insideVars = false;
        }
      } else if (insideVars) {
        const varMatches = [...line.matchAll(/\("(\w+)",\s*ValueType::(\w+),\s*(\w+)\)/g)];
        for (const vm of varMatches) {
          for (const ev of currentEvents) {
            if (!inputVarsMap[ev]) inputVarsMap[ev] = [];
            inputVarsMap[ev].push({ name: vm[1], type: vm[2], optional: vm[3] === 'false' });
          }
        }
        if (line.includes(']')) {
          insideVars = false;
        }
      }
    }
  }
  return inputVarsMap;
}
function parseAllowsCustomInputVars(effectContent) {
  const map = {};
  const match = effectContent.match(/pub fn allows_custom_input_vars\(&self\)\s*->\s*bool\s*{([\s\S]*?)^    }/m);
  if (match) {
    const body = match[1];
    const lines = body.split('\n');
    let currentEvents = [];
    
    for (let line of lines) {
      line = line.trim();
      if (line.includes('=>')) {
        const arrowIndex = line.indexOf('=>');
        const leftSide = line.substring(0, arrowIndex).trim();
        const rightSide = line.substring(arrowIndex + 2).trim();
        
        currentEvents = [];
        const eventMatches = [...leftSide.matchAll(/Self::(\w+)/g)];
        for (const em of eventMatches) {
          currentEvents.push(em[1]);
        }
        
        const isTrue = rightSide.includes('true');
        for (const ev of currentEvents) {
          map[ev] = isTrue;
        }
      }
    }
  }
  return map;
}


function scrapeTypeMappings(filePath) {
  const content = fs.readFileSync(filePath, "utf8");
  const lines = content.split("\n");
  const mapping = {};

  let insideValueType = false;
  let openBrackets = 0;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line.includes("pub fn value_type(&self) -> ValueType {")) {
      insideValueType = true;
      openBrackets = 1;
      continue;
    }

    if (insideValueType) {
      for (const char of line) {
        if (char === '{') openBrackets++;
        if (char === '}') openBrackets--;
      }
      if (openBrackets <= 0) {
        insideValueType = false;
        continue;
      }

      // Match: Self::Variant(_) => ValueType::Type,
      const match = line.match(
        /(?:Self|ValueRef|Value|MaybeReferenceValue)::(\w+)(?:\(.*\))?\s*=>\s*ValueType::(\w+)/,
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
    const memberMatch = line.match(/^"([a-z0-9_]+)"(?:\s*\|\s*"[a-z0-9_]+")*\s*=>/);
    if (memberMatch) {
      const memberName = memberMatch[1];
      const { returnTypes, onlyApplicableToMove } = extractReturnTypes(lines, i, typeMapping);

      let returnType = "Undefined";
      let itemType = null;
      if (returnTypes.has("List")) {
        for (const t of returnTypes) {
          if (t !== "List" && t !== "Undefined") {
            itemType = t;
            break;
          }
        }
        if (itemType) returnTypes.delete(itemType);
      }

      if (returnTypes.size > 0) {
        if (returnTypes.size > 1 && returnTypes.has("Undefined")) {
          returnTypes.delete("Undefined");
          returnType = Array.from(returnTypes).join(" | ") + " | Undefined";
        } else {
          returnType = Array.from(returnTypes).join(" | ");
        }
      }

      const memberData = { description: "", type: returnType };
      if (itemType) memberData.item_type = itemType;
      if (onlyApplicableToMove && currentType === "Effect") {
        memberData.only_applicable_to_move = true;
      }

      if (currentType === "global") {
        metadata.global[memberName] = memberData;
      } else {
        if (metadata.types[currentType][memberName]) {
          const existingType = metadata.types[currentType][memberName].type;
          const existingItemType = metadata.types[currentType][memberName].item_type;
          const existingMoveOnly = metadata.types[currentType][memberName].only_applicable_to_move;
          const existingActiveMoveOnly = metadata.types[currentType][memberName].only_applicable_to_active_move;
          
          if (existingItemType && !memberData.item_type) {
            memberData.item_type = existingItemType;
          }
          if (existingMoveOnly && !memberData.only_applicable_to_move) {
            memberData.only_applicable_to_move = true;
          }
          if (existingActiveMoveOnly && !memberData.only_applicable_to_active_move) {
            memberData.only_applicable_to_active_move = true;
          }
          
          if (existingType !== returnType) {
            const types = new Set([...existingType.split(" | "), ...returnType.split(" | ")]);
            if (types.size > 1 && types.has("Undefined")) {
              types.delete("Undefined");
              memberData.type = Array.from(types).join(" | ") + " | Undefined";
            } else {
              memberData.type = Array.from(types).join(" | ");
            }
          }
        }
        metadata.types[currentType][memberName] = memberData;

        if (currentType === "ActiveMove") {
          if (!metadata.types["Effect"]) metadata.types["Effect"] = {};
          const effectMemberData = { ...memberData, only_applicable_to_active_move: true };
          
          if (metadata.types["Effect"][memberName]) {
            const existingType = metadata.types["Effect"][memberName].type;
            const existingItemType = metadata.types["Effect"][memberName].item_type;
            const existingMoveOnly = metadata.types["Effect"][memberName].only_applicable_to_move;
            const existingActiveMoveOnly = metadata.types["Effect"][memberName].only_applicable_to_active_move;
            
            if (existingItemType && !effectMemberData.item_type) {
              effectMemberData.item_type = existingItemType;
            }
            if (existingMoveOnly) {
              effectMemberData.only_applicable_to_move = true;
            }
            
            if (existingType !== returnType) {
              const types = new Set([...existingType.split(" | "), ...returnType.split(" | ")]);
              if (types.size > 1 && types.has("Undefined")) {
                types.delete("Undefined");
                effectMemberData.type = Array.from(types).join(" | ") + " | Undefined";
              } else {
                effectMemberData.type = Array.from(types).join(" | ");
              }
            }
          }
          metadata.types["Effect"][memberName] = effectMemberData;
        }
      }
      docBuffer = [];
    } else if (line !== "" && !line.startsWith("//") && !line.startsWith("}")) {
      // No reset here
    }
  }

  if (metadata.types["Effect"]) {
    for (const key in metadata.types["Effect"]) {
      const member = metadata.types["Effect"][key];
      if (member.only_applicable_to_move || member.only_applicable_to_active_move) {
        if (!member.type.includes("Undefined")) {
          member.type += " | Undefined";
        }
      }
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

    const match = line.match(/^"([a-z0-9_]+)"\s*=>\s*([a-zA-Z0-9_]+)?/);
    if (match) {
      const extName = match[1];
      let intName = match[2];

      if (!intName || intName === "{" || intName === "Ok" || intName === "Some") {
        const fnMatches = [...line.matchAll(/([a-zA-Z0-9_]+)\(/g)];
        let found = false;
        for (const fnMatch of fnMatches) {
          const fnName = fnMatch[1];
          if (fnName !== "map" && fnName !== "Ok" && fnName !== "Some") {
            intName = fnName;
            found = true;
            break;
          }
        }
        
        if (!found) {
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
      }

      if (intName && intName !== "{") {
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
      let itemType = undefined;
      let returnsItemFromList = false;
      let parameters = [];
      let flags = [];
      for (let i = defLine - 1; i >= 0; i--) {
        const line = fnContents[i].trim();
        if (line.startsWith("///")) {
          const docLine = line.replace("///", "").trim();

          const itemTypeMatch = docLine.match(/@returnsitem\s*\{(.*)\}/);
          const retMatch = docLine.match(/@returns\s*\{(.*)\}/);
          const paramMatch = docLine.match(
            /@param\s*\{(.*)\}\s*(?:\[([\w\.]+)\]|([\w\.]+))\s*(.*)/,
          );
          const flagMatch = docLine.match(/@flag\s*(\w+)\s*(.*)/);

          if (docLine.includes("@returns_item_from_list")) {
            returnsItemFromList = true;
            returnType = "unknown";
          } else if (itemTypeMatch) {
            const rawItemType = itemTypeMatch[1];
            itemType = rawItemType.replace(/\[`ValueType::(\w+)`\]/g, "$1");
          } else if (retMatch) {
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
        item_type: itemType,
        returns_item_from_list: returnsItemFromList,
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
  const commonTypesMap = parseCommonCallbackTypeBitmasks(effectContent, flagsMap);

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
  const events = parseBattleEventDescriptions(effectContent);
  // 5. Parse input_vars globally
  const inputVarsMap = parseInputVars(effectContent);
  const allowsCustomVarsMap = parseAllowsCustomInputVars(effectContent);

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

        // Add variables from input_vars()
        if (inputVarsMap[eventName]) {
          const typeMap = {
            'UFraction': 'UFraction',
            'Fraction': 'Fraction',
            'Boolean': 'Boolean',
            'String': 'String',
            'Effect': 'Effect',
            'Mon': 'Mon',
            'List': 'List',
            'Object': 'Object',
            'Type': 'Type',
            'BoostTable': 'BoostTable',
            'StatTable': 'StatTable',
            'Stat': 'Stat',
            'Boost': 'Boost'
          };
          for (const v of inputVarsMap[eventName]) {
            const type = typeMap[v.type] || v.type;
            events[eventName].variables[v.name] = { type, optional: v.optional };
          }
        }

        const snakeName = events[eventName].snakeName;
        validEvents[snakeName] = {
          description: events[eventName].description,
          variables: events[eventName].variables,
          allows_custom_input_vars: allowsCustomVarsMap[eventName] || false,
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
        description: "",
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
