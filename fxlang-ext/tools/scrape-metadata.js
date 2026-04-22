const fs = require('fs');
const path = require('path');

const REPO_ROOT = path.join(__dirname, '..', '..');
const VARIABLE_RS = path.join(REPO_ROOT, 'battler', 'src', 'effect', 'fxlang', 'variable.rs');
const FUNCTIONS_RS = path.join(REPO_ROOT, 'battler', 'src', 'effect', 'fxlang', 'functions.rs');
const EFFECT_RS = path.join(REPO_ROOT, 'battler', 'src', 'effect', 'fxlang', 'effect.rs');
const OUTPUT_FILE = path.join(__dirname, '..', 'metadata.json');

function scrapeVariables(filePath) {
    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.split('\n');
    
    const metadata = {
        global: {},
        types: {}
    };

    let currentType = 'global';
    let docBuffer = [];

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();

        // Check for doc comments
        if (line.startsWith('///')) {
            docBuffer.push(line.replace('///', '').trim());
            continue;
        }

        // Check for type transitions
        // Pattern: } else if let Some(mon_handle) = value.mon_handle() {
        const typeMatch = line.match(/else if let Some\(\w+\) = value\.(\w+)_handle\(\)/);
        if (typeMatch) {
            currentType = typeMatch[1].charAt(0).toUpperCase() + typeMatch[1].slice(1);
            if (!metadata.types[currentType]) metadata.types[currentType] = {};
            docBuffer = [];
            continue;
        }

        // Check for member match arms
        // Pattern: "hp" =>
        const memberMatch = line.match(/^"([a-z0-9_]+)"\s*=>/);
        if (memberMatch) {
            const memberName = memberMatch[1];
            const description = docBuffer.join(' ');
            if (currentType === 'global') {
                metadata.global[memberName] = { description };
            } else {
                metadata.types[currentType][memberName] = { description };
            }
            docBuffer = [];
        } else if (line !== '' && !line.startsWith('//')) {
            // Clear doc buffer if we hit a non-empty line that isn't a comment/match
            // docBuffer = [];
        }
    }

    return metadata;
}

function scrapeFunctions(filePath) {
    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.split('\n');
    
    const functions = {};
    let docBuffer = [];
    let insideMatch = false;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();

        if (line.includes('match function_name {')) {
            insideMatch = true;
            continue;
        }

        if (insideMatch && line === '}') {
            insideMatch = false;
            break;
        }

        if (!insideMatch) continue;

        if (line.startsWith('///')) {
            docBuffer.push(line.replace('///', '').trim());
            continue;
        }

        const funcMatch = line.match(/^"([a-z0-9_]+)"\s*=>/);
        if (funcMatch) {
            const name = funcMatch[1];
            functions[name] = {
                description: docBuffer.join(' ')
            };
            docBuffer = [];
        } else if (line !== '' && !line.startsWith('//')) {
            docBuffer = [];
        }
    }

    return functions;
}

function scrapeEvents(filePath) {
    const content = fs.readFileSync(filePath, 'utf8');
    // Simplified: Just look for CallbackFlag usages in a future pass
    // For now, let's just return a placeholder or implement it if possible
    return {};
}

function main() {
    console.log('Scraping fxlang metadata...');
    
    const vars = scrapeVariables(VARIABLE_RS);
    const funcs = scrapeFunctions(FUNCTIONS_RS);
    const events = scrapeEvents(EFFECT_RS);

    const fullMetadata = {
        variable_members: vars.global,
        type_members: vars.types,
        functions: funcs,
        events: events
    };

    fs.writeFileSync(OUTPUT_FILE, JSON.stringify(fullMetadata, null, 2));
    console.log(`Metadata written to ${OUTPUT_FILE}`);
}

main();
