const fs = require('fs');
const md = require('./metadata.json');
const grammarPath = './syntaxes/fxlang.tmLanguage.json';
const grammar = JSON.parse(fs.readFileSync(grammarPath, 'utf8'));

// The constants pattern is at grammar.repository.constants.patterns[0].match
// Current: "\\b(true|false|undefined|stop)\\b"

const builtin = ["true", "false", "undefined", "stop"];
const allConstants = [...builtin, ...md.common_flags];
const matchRegex = "\\\\b(" + allConstants.join("|") + ")\\\\b";

grammar.repository.constants.patterns[0].match = matchRegex;

fs.writeFileSync(grammarPath, JSON.stringify(grammar, null, 2) + "\n");
console.log("Grammar patched!");
