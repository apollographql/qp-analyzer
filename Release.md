# Release process

We publish WASM build to NPM.

## Update package versions

In Cargo.toml files:
- crates/analyzer/Cargo.toml
- crates/cli/Cargo.toml
- crates/wasm/Cargo.toml

## Test WASM build
```
    (cd crates/wasm/tests; npm run build-and-test)
    node crates/wasm/scripts/fix-package-json.js
    npx crates/wasm/pkg
```

## Commit and push commits

```
git commit
git push
```

## Tag and push tags

```
git tag -vX.Y.Z
git push --tags
```
