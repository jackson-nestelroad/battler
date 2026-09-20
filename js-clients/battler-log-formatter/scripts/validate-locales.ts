import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const enFile = fs.readFileSync(path.resolve(__dirname, '../locales/en.ts'), 'utf-8');
const templateRegex = /\{\{([^}]+)\}\}/g;

let match;
let hasError = false;
while ((match = templateRegex.exec(enFile)) !== null) {
    const variable = match[1].trim();
    if (!/^[a-zA-Z0-9_]+$/.test(variable)) {
        console.error(`❌ Validation Error: Invalid template variable identifier '{{${variable}}}' found in en.ts`);
        hasError = true;
    }
}

if (hasError) {
    process.exit(1);
} else {
    console.log(`✅ All template variables in en.ts are valid!`);
    process.exit(0);
}
