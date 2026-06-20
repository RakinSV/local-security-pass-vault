#!/usr/bin/env node
'use strict';
// Verifies extension build integrity: all top-level .js files must be referenced
// either in manifest.json or in a popup/options HTML file.
// Outputs SHA-256 checksums for audit log.  Run after `npm run build`.
const fs     = require('fs');
const path   = require('path');
const crypto = require('crypto');

const distDir  = path.join(__dirname, '..', 'dist');
const manifest = JSON.parse(fs.readFileSync(path.join(distDir, 'manifest.json'), 'utf8'));

// ── 1. Collect JS files declared directly in the manifest ─────────────────────
const expectedTopLevel = new Set();

if (manifest.background?.service_worker) {
  expectedTopLevel.add(manifest.background.service_worker);
}
(manifest.content_scripts || []).forEach(cs =>
  (cs.js || []).forEach(j => expectedTopLevel.add(j))
);

// ── 2. Collect JS files referenced from popup / options HTML files ─────────────
function collectHtmlScripts(htmlRelPath) {
  const htmlAbs = path.join(distDir, htmlRelPath);
  if (!fs.existsSync(htmlAbs)) return;
  const content = fs.readFileSync(htmlAbs, 'utf8');
  // Match <script src="..."> and <script type="module" src="...">
  for (const m of content.matchAll(/\bsrc=["']([^"']+\.js)["']/g)) {
    const raw = m[1];
    // Strip leading ./ or / to get a dist-relative path
    const rel = raw.replace(/^\.\//, '').replace(/^\//, '');
    if (!rel.includes('/')) expectedTopLevel.add(rel);  // only top-level
  }
}

if (manifest.action?.default_popup) collectHtmlScripts(manifest.action.default_popup);
if (manifest.options_ui?.page)      collectHtmlScripts(manifest.options_ui.page);
if (manifest.options_page)          collectHtmlScripts(manifest.options_page);

// ── 3. Find all .js files recursively ─────────────────────────────────────────
function findJs(dir, base) {
  const results = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...findJs(full, base));
    } else if (entry.name.endsWith('.js')) {
      results.push(path.relative(base, full).replace(/\\/g, '/'));
    }
  }
  return results;
}

const allJs = findJs(distDir, distDir);

// ── 4. Output SHA-256 checksums for audit log ──────────────────────────────────
console.log('=== Extension build checksums ===');
for (const f of allJs.slice().sort()) {
  const data = fs.readFileSync(path.join(distDir, f));
  const hash = crypto.createHash('sha256').update(data).digest('hex');
  console.log(hash + '  ' + f);
}
console.log('');

// ── 5. Verify every top-level .js is referenced (manifest or HTML) ─────────────
const topLevel   = allJs.filter(f => !f.includes('/'));
const unexpected = topLevel.filter(f => !expectedTopLevel.has(f));
if (unexpected.length > 0) {
  console.error('ERROR: top-level .js files not referenced in manifest or popup HTML:');
  unexpected.forEach(f => console.error('  ' + f));
  process.exit(1);
}

// ── 6. Verify all manifest-referenced files actually exist in dist ─────────────
for (const expected of expectedTopLevel) {
  if (!fs.existsSync(path.join(distDir, expected))) {
    console.error('ERROR: manifest-referenced file missing from dist: ' + expected);
    process.exit(1);
  }
}

console.log('✓ All ' + topLevel.length + ' top-level .js files are accounted for (' +
  expectedTopLevel.size + ' expected sources).');
