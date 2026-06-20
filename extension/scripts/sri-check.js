#!/usr/bin/env node
'use strict';
// Verifies that all top-level .js files in dist/ are referenced in manifest.json
// and outputs SHA-256 checksums for audit.  Run after `npm run build`.
const fs     = require('fs');
const path   = require('path');
const crypto = require('crypto');

const distDir  = path.join(__dirname, '..', 'dist');
const manifest = JSON.parse(fs.readFileSync(path.join(distDir, 'manifest.json'), 'utf8'));

// Collect files declared by the manifest
const expectedTopLevel = new Set();
if (manifest.background?.service_worker) {
  expectedTopLevel.add(manifest.background.service_worker);
}
(manifest.content_scripts || []).forEach(cs =>
  (cs.js || []).forEach(j => expectedTopLevel.add(j))
);

// Find all .js files recursively
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

// Output SHA-256 checksums
console.log('=== Extension build checksums ===');
for (const f of allJs.slice().sort()) {
  const data = fs.readFileSync(path.join(distDir, f));
  const hash = crypto.createHash('sha256').update(data).digest('hex');
  console.log(hash + '  ' + f);
}
console.log('');

// Check top-level .js files are all in the manifest
const topLevel   = allJs.filter(f => !f.includes('/'));
const unexpected = topLevel.filter(f => !expectedTopLevel.has(f));
if (unexpected.length > 0) {
  console.error('ERROR: top-level .js files not referenced in manifest:');
  unexpected.forEach(f => console.error('  ' + f));
  process.exit(1);
}

// Check all manifest-referenced files actually exist
for (const expected of expectedTopLevel) {
  if (!fs.existsSync(path.join(distDir, expected))) {
    console.error('ERROR: manifest-referenced file missing from dist: ' + expected);
    process.exit(1);
  }
}

console.log('✓ All ' + topLevel.length + ' top-level .js files are referenced in manifest.');
