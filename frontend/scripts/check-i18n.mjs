import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import ts from 'typescript';

const projectRoot = path.resolve(import.meta.dirname, '..');
const moduleCache = new Map();

function loadTypeScriptModule(filePath) {
  const resolvedPath = path.resolve(filePath);
  if (moduleCache.has(resolvedPath)) return moduleCache.get(resolvedPath);

  const source = fs.readFileSync(resolvedPath, 'utf8');
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
      esModuleInterop: true,
    },
    fileName: resolvedPath,
  }).outputText;

  const module = { exports: {} };
  moduleCache.set(resolvedPath, module.exports);
  const execute = vm.runInThisContext(
    `(function (require, module, exports) { ${output}\n})`,
    { filename: resolvedPath }
  );
  const localRequire = (specifier) => {
    if (!specifier.startsWith('.')) {
      throw new Error(`Unsupported import in locale resource: ${specifier}`);
    }
    const importedPath = path.resolve(path.dirname(resolvedPath), specifier);
    return loadTypeScriptModule(
      path.extname(importedPath) ? importedPath : `${importedPath}.ts`
    );
  };
  execute(localRequire, module, module.exports);
  moduleCache.set(resolvedPath, module.exports);
  return module.exports;
}

function placeholders(value) {
  return [...value.matchAll(/{{\s*([\w.-]+)(?:\s*,[^}]*)?\s*}}/g)]
    .map((match) => match[1])
    .sort();
}

function compareLocales(source, target, location = '') {
  const errors = [];
  const sourceKeys = Object.keys(source).sort();
  const targetKeys = Object.keys(target).sort();

  for (const key of sourceKeys.filter((key) => !targetKeys.includes(key))) {
    errors.push(`Missing zh-CN key: ${location}${key}`);
  }
  for (const key of targetKeys.filter((key) => !sourceKeys.includes(key))) {
    errors.push(`Extra zh-CN key: ${location}${key}`);
  }

  for (const key of sourceKeys.filter((key) => targetKeys.includes(key))) {
    const nextLocation = `${location}${key}`;
    const sourceValue = source[key];
    const targetValue = target[key];
    const sourceIsObject = sourceValue !== null && typeof sourceValue === 'object';
    const targetIsObject = targetValue !== null && typeof targetValue === 'object';

    if (sourceIsObject !== targetIsObject) {
      errors.push(`Type mismatch: ${nextLocation}`);
      continue;
    }
    if (sourceIsObject) {
      errors.push(...compareLocales(sourceValue, targetValue, `${nextLocation}.`));
      continue;
    }
    if (typeof sourceValue !== 'string' || typeof targetValue !== 'string') {
      errors.push(`Locale leaf must be a string: ${nextLocation}`);
      continue;
    }

    const sourcePlaceholders = placeholders(sourceValue);
    const targetPlaceholders = placeholders(targetValue);
    if (sourcePlaceholders.join('\0') !== targetPlaceholders.join('\0')) {
      errors.push(
        `Interpolation mismatch: ${nextLocation} ` +
        `(en-US: ${sourcePlaceholders.join(', ') || 'none'}; ` +
        `zh-CN: ${targetPlaceholders.join(', ') || 'none'})`
      );
    }
  }

  return errors;
}

const { resources } = loadTypeScriptModule(
  path.join(projectRoot, 'src/i18n/resources.ts')
);
const errors = compareLocales(resources['en-US'], resources['zh-CN']);
const { resolveLocale } = loadTypeScriptModule(
  path.join(projectRoot, 'src/i18n/locale.ts')
);

const localeCases = [
  [null, 'zh-CN', 'zh-CN'],
  [null, 'zh-Hans', 'zh-CN'],
  [null, 'en-US', 'en-US'],
  ['en-US', 'zh-CN', 'en-US'],
  ['zh-CN', 'en-US', 'zh-CN'],
  ['unsupported', 'fr-FR', 'en-US'],
];
for (const [stored, system, expected] of localeCases) {
  const actual = resolveLocale(stored, system);
  if (actual !== expected) {
    errors.push(
      `Locale resolution mismatch: stored=${stored}, system=${system}, ` +
      `expected=${expected}, actual=${actual}`
    );
  }
}

if (errors.length > 0) {
  console.error(errors.join('\n'));
  process.exitCode = 1;
} else {
  console.log('i18n resources: keys and interpolation parameters match.');
}
