const fs = require('fs');
const content = fs.readFileSync('../battler/src/effect/fxlang/eval.rs', 'utf8');
const varRegex = /self\.vars\s*\.\s*set\("(\w+)",\s*Value::(\w+).*\)\?/g;
let match;
while ((match = varRegex.exec(content)) !== null) {
    console.log(match[1], match[2]);
}
