const fs = require('fs');
const extension = fs.readFileSync('src/extension.ts', 'utf8');
const hoverProviderMatches = extension.match(/provideHover.*?return items;/s);
// We can't really run it in node easily, but we can verify the logic.
