import fs from 'node:fs';
import path from 'node:path';

const ID_RE = /^[A-Za-z_$][A-Za-z0-9_$]*$/;
const UI_BY_TYPE = {
  number: new Set(['slider', 'input']),
  color: new Set(['color']),
  select: new Set(['select']),
  boolean: new Set(['toggle', 'checkbox']),
  text: new Set(['text', 'textarea']),
};

function fail(message) {
  console.error(`[epris:validate] ${message}`);
  process.exitCode = 1;
}

function ok(message) {
  console.log(`[epris:validate] ${message}`);
}

function readJson(filePath) {
  const text = fs.readFileSync(filePath, 'utf8');
  return JSON.parse(text);
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function validateControls(controlsPath) {
  const spec = readJson(controlsPath);
  if (!isPlainObject(spec)) {
    fail(`epris-controls.json must be an object`);
    return null;
  }
  if (spec.schemaVersion !== 1) {
    fail(`epris-controls.json schemaVersion must be 1`);
  }
  if (!Array.isArray(spec.controls)) {
    fail(`epris-controls.json controls must be an array`);
    return null;
  }

  const ids = new Set();
  for (const c of spec.controls) {
    if (!isPlainObject(c)) {
      fail(`control must be an object`);
      continue;
    }
    if (typeof c.id !== 'string' || !c.id) {
      fail(`control.id must be a non-empty string`);
      continue;
    }
    if (!ID_RE.test(c.id)) {
      fail(`control.id '${c.id}' is not a valid JS identifier`);
    }
    if (ids.has(c.id)) {
      fail(`duplicate control id '${c.id}'`);
    }
    ids.add(c.id);

    if (typeof c.type !== 'string') {
      fail(`control '${c.id}' missing type`);
      continue;
    }
    if (!['number', 'color', 'select', 'boolean', 'text'].includes(c.type)) {
      fail(`control '${c.id}' has invalid type '${c.type}'`);
    }

    if (c.ui != null) {
      if (typeof c.ui !== 'string') {
        fail(`control '${c.id}' ui must be a string`);
      } else {
        const allowed = UI_BY_TYPE[c.type];
        if (allowed && !allowed.has(c.ui)) {
          fail(
            `control '${c.id}' has invalid ui '${c.ui}' for type '${c.type}' (allowed: ${Array.from(allowed).join(', ')})`,
          );
        }
      }
    }

    if (c.type === 'select') {
      if (!Array.isArray(c.options) || c.options.length === 0) {
        fail(`select control '${c.id}' must have non-empty options`);
      }
    }

    if (c.type === 'number') {
      if (c.min != null && typeof c.min !== 'number') fail(`number control '${c.id}' min must be number`);
      if (c.max != null && typeof c.max !== 'number') fail(`number control '${c.id}' max must be number`);
      if (c.step != null && typeof c.step !== 'number') fail(`number control '${c.id}' step must be number`);
    }

    if (c.objectId != null) {
      if (typeof c.objectId !== 'string' || !ID_RE.test(c.objectId)) {
        fail(`control '${c.id}' has invalid objectId (must be JS identifier)`);
      }
    }
  }

  return { spec, ids, controls: spec.controls };
}

function validateProps(propsPath, controlsIds, controls) {
  const propsFile = readJson(propsPath);
  if (!isPlainObject(propsFile)) {
    fail(`epris-props.json must be an object`);
    return null;
  }
  if (propsFile.schemaVersion !== 1) {
    fail(`epris-props.json schemaVersion must be 1`);
  }
  if (!isPlainObject(propsFile.values)) {
    fail(`epris-props.json values must be an object`);
    return null;
  }

  for (const id of controlsIds) {
    if (!(id in propsFile.values)) {
      fail(`epris-props.json missing value for '${id}'`);
    }
  }

  for (const c of controls) {
    const v = propsFile.values[c.id];
    if (v == null) continue;

    if (c.type === 'number') {
      if (typeof v !== 'number' || !Number.isFinite(v)) {
        fail(`value for '${c.id}' must be a finite number`);
        continue;
      }
      if (typeof c.min === 'number' && v < c.min) fail(`value for '${c.id}' must be >= ${c.min}`);
      if (typeof c.max === 'number' && v > c.max) fail(`value for '${c.id}' must be <= ${c.max}`);
      continue;
    }

    if (c.type === 'boolean') {
      if (typeof v !== 'boolean') fail(`value for '${c.id}' must be boolean`);
      continue;
    }

    if (c.type === 'text') {
      if (typeof v !== 'string') fail(`value for '${c.id}' must be string`);
      continue;
    }

    if (c.type === 'color') {
      if (typeof v !== 'string') fail(`value for '${c.id}' must be string (hex)`);
      continue;
    }

    if (c.type === 'select') {
      if (typeof v !== 'string') {
        fail(`value for '${c.id}' must be string (selected option value)`);
        continue;
      }
      const opts = Array.isArray(c.options) ? c.options : [];
      const allowed = new Set(opts.map((o) => (typeof o === 'string' ? o : o.value)));
      if (allowed.size > 0 && !allowed.has(v)) {
        fail(`value for '${c.id}' must be one of the select options`);
      }
    }
  }

  return propsFile;
}

function validateRootTsx(rootPath) {
  const text = fs.readFileSync(rootPath, 'utf8');
  if (!text.includes('epris-props.json')) {
    fail(`Root.tsx must import './epris-props.json'`);
  }
  if (!text.includes('defaultProps')) {
    fail(`Root.tsx must pass defaultProps to Composition`);
  }
}

const workspaceRoot = process.cwd();
const srcDir = path.join(workspaceRoot, 'src');
const controlsPath = path.join(srcDir, 'epris-controls.json');
const propsPath = path.join(srcDir, 'epris-props.json');
const rootPath = path.join(srcDir, 'Root.tsx');

const hasControls = fs.existsSync(controlsPath);
const hasProps = fs.existsSync(propsPath);

if (!hasControls && !hasProps) {
  ok('No epris-controls.json/epris-props.json found (skipping).');
  process.exit(0);
}

if (hasControls !== hasProps) {
  fail('Both src/epris-controls.json and src/epris-props.json must exist together.');
}

const res = validateControls(controlsPath);
if (res) validateProps(propsPath, res.ids, res.controls);
if (fs.existsSync(rootPath)) validateRootTsx(rootPath);

if (process.exitCode && process.exitCode !== 0) {
  process.exit(process.exitCode);
}

ok('Contract OK.');
