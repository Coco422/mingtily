import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const frontendDirectory = fileURLToPath(new URL('../', import.meta.url));

async function readConfig(name) {
  const content = await readFile(`${frontendDirectory}src-tauri/${name}`, 'utf8');
  return JSON.parse(content);
}

function assertEqual(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
  }
}

const local = await readConfig('tauri.conf.json');
const release = await readConfig('tauri.release.conf.json');

assertEqual(local.productName, 'Mingtily Dev', 'Local product name must be visibly isolated');
assertEqual(
  local.identifier,
  'com.mingcheng.mingtily.dev',
  'Local app data must use the development identifier'
);
assertEqual(local.app?.windows?.[0]?.title, 'Mingtily Dev', 'Local window title must identify the development app');
assertEqual(local.bundle?.createUpdaterArtifacts, false, 'Local builds must not create updater artifacts');

assertEqual(release.productName, 'Mingtily', 'Release product name must remain canonical');
assertEqual(
  release.identifier,
  'com.mingcheng.mingtily',
  'Release app must use the production identifier'
);
assertEqual(release.app?.windows?.[0]?.title, 'Mingtily', 'Release window title must remain canonical');
assertEqual(release.bundle?.createUpdaterArtifacts, true, 'Tagged releases must create updater artifacts');

for (const key of ['width', 'height', 'resizable', 'fullscreen', 'theme', 'decorations']) {
  assertEqual(
    release.app?.windows?.[0]?.[key],
    local.app?.windows?.[0]?.[key],
    `Release window setting ${key} must match the locally tested window`
  );
}

console.log('App identity boundaries are valid.');
