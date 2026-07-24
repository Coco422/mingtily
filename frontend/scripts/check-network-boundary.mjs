import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const frontendDir = path.resolve(scriptDir, '..');
const repositoryDir = path.resolve(frontendDir, '..');

const failures = [];

function read(relativePath) {
  return fs.readFileSync(path.join(repositoryDir, relativePath), 'utf8');
}

function assertAbsent(label, content, patterns) {
  for (const pattern of patterns) {
    if (pattern.test(content)) {
      failures.push(`${label}: matched ${pattern}`);
    }
  }
}

const rustEntry = read('frontend/src-tauri/src/lib.rs');
const setupStart = rustEntry.indexOf('.setup(');
const setupEnd = rustEntry.indexOf('.on_window_event', setupStart);
if (setupStart === -1 || setupEnd === -1) {
  failures.push('Rust startup: could not locate the Tauri setup block');
} else {
  assertAbsent('Rust startup', rustEntry.slice(setupStart, setupEnd), [
    /https?:\/\//i,
    /reqwest/i,
    /download_model/i,
    /get_(openai|anthropic|groq|openrouter)_models/i,
    /api_process_transcript/i,
  ]);
}

const coldStartFrontendFiles = [
  'frontend/src/app/layout.tsx',
  'frontend/src/app/page.tsx',
  'frontend/src/contexts/ConfigContext.tsx',
  'frontend/src/components/MainContent/index.tsx',
  'frontend/src/components/Sidebar/SidebarProvider.tsx',
];
const remoteCommand = /invoke(?:<[^>]+>)?\(\s*['"](?:open_external_url|get_(?:openai|anthropic|groq|openrouter)_models|api_process_transcript|\w+_download_model)['"]/i;
for (const relativePath of coldStartFrontendFiles) {
  assertAbsent(relativePath, read(relativePath), [
    /\bfetch\s*\(/,
    /https?:\/(?!\/(?:localhost|127\.0\.0\.1|\[::1\]))/i,
    remoteCommand,
  ]);
}

const runtimeSources = [
  ...fs.readdirSync(path.join(repositoryDir, 'frontend/src'), { recursive: true }),
  ...fs.readdirSync(path.join(repositoryDir, 'frontend/src-tauri/src'), { recursive: true }),
]
  .filter((entry) => typeof entry === 'string' && /\.(?:rs|ts|tsx)$/.test(entry))
  .map((entry) => entry.toLowerCase());
for (const forbiddenName of ['posthog', 'analytics.json', 'tauri-plugin-updater']) {
  if (runtimeSources.some((entry) => entry.includes(forbiddenName))) {
    failures.push(`Runtime source tree still contains forbidden entry: ${forbiddenName}`);
  }
}

if (failures.length > 0) {
  console.error('Network-boundary source audit failed:');
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log('Network-boundary source audit passed.');
