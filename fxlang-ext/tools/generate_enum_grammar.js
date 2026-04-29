const fs = require('fs');
const path = require('path');

const BOOST_RS = '/Users/jackson/Code/GitHub/pokemon/battler-data/src/moves/boost.rs';
const TYPE_RS = '/Users/jackson/Code/GitHub/pokemon/battler-data/src/mons/type.rs';
const GRAMMAR_JSON = '/Users/jackson/Code/GitHub/pokemon/fxlang-ext/syntaxes/fxlang-injection.tmLanguage.json';

function extractVariants(filePath, enumName) {
    const content = fs.readFileSync(filePath, 'utf8');
    const words = new Set();
    
    // Find the enum block
    const enumRegex = new RegExp(`pub enum ${enumName}\\s*{([^}]+)}`);
    const enumMatch = content.match(enumRegex);
    if (!enumMatch) return [];
    
    const enumBlock = enumMatch[1];
    
    // Match #[string = "..."]
    const stringMatches = enumBlock.matchAll(/#\[string = "([^"]+)"\]/g);
    for (const match of stringMatches) {
        words.add(match[1]);
    }
    
    // Match #[alias = "..."]
    const aliasMatches = enumBlock.matchAll(/#\[alias = "([^"]+)"\]/g);
    for (const match of aliasMatches) {
        words.add(match[1]);
    }
    
    // Match enum variants themselves (fallback/capitalized names)
    const variantMatches = enumBlock.matchAll(/^\s+([A-Z][a-zA-Z0-9]+),/gm);
    for (const match of variantMatches) {
        words.add(match[1]);
    }

    // Filter out punctuation
    const filtered = Array.from(words).filter(w => !/[^a-zA-Z0-9\s\-_]/.test(w));
    return filtered;
}

const ALLOWED_BOOSTS = ['atk', 'def', 'spa', 'spd', 'spe', 'acc', 'eva', 'spatk', 'spdef'];
let boostVariants = extractVariants(BOOST_RS, 'Boost');
// Keep shortened versions only
boostVariants = boostVariants.filter(v => ALLOWED_BOOSTS.includes(v.toLowerCase()));

let typeVariants = extractVariants(TYPE_RS, 'Type');
// Lowercase all types
typeVariants = typeVariants.map(v => v.toLowerCase());

const allVariants = [...new Set([...boostVariants, ...typeVariants])];

console.log('Extracted variants:', allVariants);

// Escape variants for regex
const escapedVariants = allVariants.map(v => v.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&')).join('|');

// Read grammar
const grammar = JSON.parse(fs.readFileSync(GRAMMAR_JSON, 'utf8'));

// Add to repository using a single regex with backreference
grammar.repository.special_variants = {
    "patterns": [
        {
            "match": `(?i)('?)\\b(${escapedVariants})\\b\\1`,
            "name": "constant.character.escape"
        }
    ]
};

// Add to statement patterns if not present
const statementPatterns = grammar.repository.statement.patterns;
const hasSpecialVariants = statementPatterns.some(p => p.include === '#special_variants');
if (!hasSpecialVariants) {
    statementPatterns.unshift({ "include": "#special_variants" });
}

fs.writeFileSync(GRAMMAR_JSON, JSON.stringify(grammar, null, 2), 'utf8');
console.log('Updated grammar with backreferences successfully.');
