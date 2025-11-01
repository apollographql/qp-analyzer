const fs = require('fs');
const path = require('path');

const pkgPath = path.join(__dirname, '../pkg/package.json');

// Read as text
const text = fs.readFileSync(pkgPath, 'utf8');

// Parse JSON
const pkg = JSON.parse(text);

// Modify package name
pkg.name = '@apollo/qp-analyzer';

// Write JSON back
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8');
