import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { ALLOWED_CONTEXT_VARS } from '../src/mapper.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const enFile = fs.readFileSync(path.resolve(__dirname, '../locales/en.ts'), 'utf-8');
const templateRegex = /\{\{([A-Z_]+)\}\}/g;

let match;
let hasError = false;
while ((match = templateRegex.exec(enFile)) !== null) {
    const variable = match[1];
    if (!ALLOWED_CONTEXT_VARS.includes(variable)) {
        console.error(`❌ Validation Error: Invalid template variable '{{${variable}}}' found in en.ts`);
        hasError = true;
    }
}

if (hasError) {
    console.error(`Valid context variables are: ${ALLOWED_CONTEXT_VARS.join(', ')}`);
    process.exit(1);
} else {
    console.log(`✅ All template variables in en.ts are valid!`);
    process.exit(0);
}
