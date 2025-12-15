const fs = require('fs');
const path = require('path');

const pkgPath = path.join(__dirname, '../pkg/package.json');
const cliSrcPath = path.join(__dirname, '../src-js/cli.js');
const cliPkgPath = path.join(__dirname, '../pkg/cli.js');

// Read as text
const text = fs.readFileSync(pkgPath, 'utf8');

// Parse JSON
const pkg = JSON.parse(text);

// Modify package name
pkg.name = '@apollo/qp-analyzer';

// Add bin field for CLI entry point
pkg.bin = {
  'qp-analyzer': './cli.js'
};

// Write JSON back
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8');

// Copy CLI file to pkg directory
fs.copyFileSync(cliSrcPath, cliPkgPath);

// Make CLI file executable
fs.chmodSync(cliPkgPath, 0o755);
