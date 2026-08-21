import { en } from './locales/en.js';
import fs from 'fs';

const logs = JSON.parse(fs.readFileSync('tests/logs-matrix.json', 'utf-8'));
const titlesInLogs = new Set();
for (const log of logs) {
    titlesInLogs.add(log.split('|')[0]);
}

const titlesInEn = new Set();
for (const key of Object.keys(en)) {
    titlesInEn.add(key.split('__')[0]);
}

const missing = [];
for (const title of titlesInEn) {
    if (!titlesInLogs.has(title)) {
        missing.push(title);
    }
}

console.log('Titles in en.ts not hit by logs-matrix.json:');
if (missing.length === 0) {
    console.log('None! 100% coverage.');
} else {
    missing.forEach(m => console.log(m));
}
