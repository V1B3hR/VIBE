const fs = require('fs');
const content = fs.readFileSync('c:/Users/brigh/.gemini/antigravity/scratch/VIBE/src-tauri/src/lib.rs', 'utf8');
const regex = /#\[tauri::command\]\s*(?:async\s+)?fn\s+([a-zA-Z0-9_]+)/g;
let match;
const commands = [];
while ((match = regex.exec(content)) !== null) {
    commands.push(match[1]);
}
console.log(commands.join('\n'));
