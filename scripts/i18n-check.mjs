#!/usr/bin/env node
/**
 * i18n consistency check (goal.md §5bis-A) — `pnpm i18n:check`.
 * Compares every locale with the reference `en.json`:
 *  - missing / orphan keys
 *  - plural objects: every CLDR category of the locale must be present
 *  - placeholders: a translation may only use placeholders known in English
 * Exits with code 1 on any error.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'ui', 'src', 'lib', 'i18n', 'locales');
const REFERENCE = 'en';
const ROMANCE = new Set(['fr', 'es', 'it', 'pt', 'ca']);

const isPlural = (v) => v !== null && typeof v === 'object' && typeof v.other === 'string';

/** Flattens to { 'a.b': string | pluralObject }. */
function flatten(tree, prefix = '', out = {}) {
  for (const [k, v] of Object.entries(tree)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === 'string' || isPlural(v)) out[key] = v;
    else flatten(v, key, out);
  }
  return out;
}

const placeholders = (s) => new Set([...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]));
const forms = (v) => (typeof v === 'string' ? [v] : Object.values(v));

const load = (code) => flatten(JSON.parse(readFileSync(join(DIR, `${code}.json`), 'utf8')));

const locales = readdirSync(DIR)
  .filter((f) => f.endsWith('.json'))
  .map((f) => f.slice(0, -5));

const ref = load(REFERENCE);
const errors = [];

for (const code of locales) {
  const cat = load(code);
  const categories = new Intl.PluralRules(code).resolvedOptions().pluralCategories;

  for (const [key, refValue] of Object.entries(ref)) {
    const value = cat[key];
    if (value === undefined) {
      errors.push(`${code}: missing key "${key}"`);
      continue;
    }
    if (isPlural(refValue) !== isPlural(value)) {
      errors.push(`${code}: "${key}" must be ${isPlural(refValue) ? 'a plural object' : 'a string'}`);
      continue;
    }
    if (isPlural(value)) {
      for (const c of categories) {
        // In Romance languages "many" only covers millions ("1 000 000 de…"): optional.
        const optional = c === 'many' && ROMANCE.has(code);
        if (!optional && value[c] === undefined) errors.push(`${code}: "${key}" lacks plural form "${c}"`);
      }
      for (const c of Object.keys(value)) {
        if (!categories.includes(c)) errors.push(`${code}: "${key}" has unknown plural form "${c}"`);
      }
    }
    const allowed = new Set(forms(refValue).flatMap((f) => [...placeholders(f)]));
    for (const f of forms(value)) {
      for (const p of placeholders(f)) {
        if (!allowed.has(p)) errors.push(`${code}: "${key}" uses unknown placeholder {${p}}`);
      }
    }
  }
  for (const key of Object.keys(cat)) {
    if (!(key in ref)) errors.push(`${code}: orphan key "${key}" (absent from ${REFERENCE}.json)`);
  }
}

if (errors.length) {
  console.error(`i18n check failed — ${errors.length} error(s):`);
  for (const e of errors) console.error(`  ✗ ${e}`);
  process.exit(1);
}
console.log(`i18n check passed — ${locales.length} locales, ${Object.keys(ref).length} keys each.`);
