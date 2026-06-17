## What does this PR do?

<!-- One paragraph summary -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Security fix / improvement
- [ ] Refactor / cleanup
- [ ] Documentation
- [ ] Build / CI

## Testing done

- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `npx tsc --noEmit` passes (extension + desktop)
- [ ] Tested manually on: <!-- OS + browser -->

## Security checklist (fill in if touching crypto or extension code)

- [ ] No key material logged
- [ ] `sodium_memzero()` used after key use
- [ ] New nonce generated per encrypt
- [ ] No `fetch()`/`XHR` added to extension
- [ ] Domain comparison uses eTLD+1

## Related issues

Closes #
