const md = require('./metadata.json');
const type = 'Mon';
const methods = [];
for (const [name, data] of Object.entries(md.functions)) {
    const firstParam = data.parameters[0];
    if (firstParam && firstParam.optional && (firstParam.type === type || firstParam.type === 'Any')) {
        methods.push(name);
    }
}
console.log("Pseudo-methods for Mon:", methods.join(', '));
