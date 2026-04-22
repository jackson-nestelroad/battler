import os
import json
import re

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
VARIABLE_RS = os.path.join(REPO_ROOT, 'battler', 'src', 'effect', 'fxlang', 'variable.rs')
FUNCTIONS_RS = os.path.join(REPO_ROOT, 'battler', 'src', 'effect', 'fxlang', 'functions.rs')
EFFECT_RS = os.path.join(REPO_ROOT, 'battler', 'src', 'effect', 'fxlang', 'effect.rs')
OUTPUT_FILE = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), 'metadata.json')

def scrape_variables(file_path):
    if not os.path.exists(file_path):
        return {"global": {}, "types": {}}
    
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    metadata = {
        "global": {},
        "types": {}
    }

    current_type = 'global'
    doc_buffer = []

    for line in lines:
        stripped = line.strip()

        # Check for doc comments
        if stripped.startswith('///'):
            doc_buffer.append(stripped.replace('///', '', 1).strip())
            continue

        # Check for type transitions
        type_match = re.search(r'else if let Some\(\w+\) = value\.(\w+)_handle\(\)', stripped)
        if type_match:
            type_name = type_match.group(1)
            current_type = type_name[0].upper() + type_name[1:]
            if current_type not in metadata["types"]:
                metadata["types"][current_type] = {}
            doc_buffer = []
            continue

        # Check for member match arms
        member_match = re.match(r'^"([a-z0-9_]+)"\s*=>', stripped)
        if member_match:
            member_name = member_match.group(1)
            description = " ".join(doc_buffer)
            if current_type == 'global':
                metadata["global"][member_name] = {"description": description}
            else:
                metadata["types"][current_type][member_name] = {"description": description}
            doc_buffer = []
        elif stripped != '' and not stripped.startswith('//') and not stripped.startswith('#['):
            # Only reset if it's substantial code that isn't a match arm
            if '{' in stripped or '(' in stripped or ';' in stripped:
                doc_buffer = []

    return metadata

def scrape_functions(file_path):
    if not os.path.exists(file_path):
        return {}
    
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    functions = {}
    doc_buffer = []
    inside_match = False

    for line in lines:
        stripped = line.strip()

        if 'match function_name {' in stripped:
            inside_match = True
            continue

        if inside_match and stripped == '}':
            inside_match = False
            break

        if not inside_match:
            continue

        if stripped.startswith('///'):
            doc_buffer.append(stripped.replace('///', '', 1).strip())
            continue

        func_match = re.match(r'^"([a-z0-9_]+)"\s*=>', stripped)
        if func_match:
            name = func_match.group(1)
            functions[name] = {
                "description": " ".join(doc_buffer)
            }
            doc_buffer = []
        elif stripped != '' and not stripped.startswith('//') and not stripped.startswith('#['):
            doc_buffer = []

    return functions

def scrape_events(file_path):
    if not os.path.exists(file_path):
        return {}
    
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    # First, scrape event docstrings
    event_docs = {}
    doc_buffer = []
    inside_enum = False
    for line in lines:
        stripped = line.strip()
        if 'pub enum BattleEvent {' in stripped:
            inside_enum = True
            continue
        if inside_enum and stripped == '}':
            inside_enum = False
            continue
        if not inside_enum: continue
        
        if stripped.startswith('///'):
            doc_buffer.append(stripped.replace('///', '', 1).strip())
        elif '#[string = "' in stripped:
            event_name = re.search(r'#\[string = "([^"]+)"\]', stripped).group(1)
            event_docs[event_name] = " ".join(doc_buffer)
            doc_buffer = []
        elif stripped == '' or stripped.startswith('//'):
            pass
        else:
            doc_buffer = []

    # Second, scrape input vars
    event_vars = {}
    inside_input_vars = False
    for line in lines:
        stripped = line.strip()
        if 'fn input_vars(&self)' in stripped:
            inside_input_vars = True
            continue
        if inside_input_vars and stripped == '}':
            inside_input_vars = False
            continue
        if not inside_input_vars: continue

        # Match: Self::AccuracyExempt => &[("base_power", ValueType::UFraction, true)]
        # This is complex to regex, let's simplify
        pass

    # For now, let's just return the event names and their docs
    events = {}
    for name, doc in event_docs.items():
        events[name] = {"description": doc}
    
    return events

def main():
    print('Scraping fxlang metadata...')
    
    vars_data = scrape_variables(VARIABLE_RS)
    funcs_data = scrape_functions(FUNCTIONS_RS)
    events_data = scrape_events(EFFECT_RS)

    full_metadata = {
        "variable_members": vars_data["global"],
        "type_members": vars_data["types"],
        "functions": funcs_data,
        "events": events_data
    }

    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        json.dump(full_metadata, f, indent=2)
    
    print(f'Metadata written to {OUTPUT_FILE}')

if __name__ == '__main__':
    main()
