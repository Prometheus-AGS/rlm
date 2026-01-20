# Pull Request

## Description

<!-- Brief description of the changes -->

## Type of Change

- [ ] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] ✨ New feature (non-breaking change which adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] 📚 Documentation update
- [ ] 🔧 Refactoring (no functional changes)
- [ ] ⚡ Performance improvement
- [ ] 🧪 Test addition or improvement
- [ ] 🔒 Security fix
- [ ] 🏗️ Build system or CI/CD changes

## Related Issues

<!-- Link to any related issues -->
Fixes #(issue number)

## Changes Made

<!-- Detailed description of what was changed -->

-
-
-

## Testing

<!-- Describe the testing you've done -->

### Test Results

- [ ] All existing tests pass
- [ ] New tests added and passing
- [ ] Integration tests pass
- [ ] Documentation examples work
- [ ] Manual testing completed

### Test Commands Used

```bash
# List the commands you used to test
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## Performance Impact

<!-- If applicable, describe any performance implications -->

- [ ] No performance impact
- [ ] Performance improvement
- [ ] Performance regression (justified below)
- [ ] Performance impact unknown/not measured

## Documentation

- [ ] Code is self-documenting
- [ ] New public APIs are documented
- [ ] README updated if needed
- [ ] CHANGELOG.md updated if needed
- [ ] API documentation updated

## Security Considerations

- [ ] No security implications
- [ ] Security review completed
- [ ] Potential security impact (describe below)

## Breaking Changes

<!-- If this is a breaking change, list what breaks and migration steps -->

- [ ] No breaking changes
- [ ] Breaking changes (listed below with migration guide)

### Breaking Change Details

<!-- If applicable, describe what breaks and how to migrate -->

## Checklist

### Code Quality

- [ ] Code follows the project's style guidelines
- [ ] Self-review of code completed
- [ ] Code is commented where necessary
- [ ] No dead code or unnecessary comments
- [ ] Error handling is appropriate
- [ ] Logging is appropriate (no `println!` in library code)

### Rust Specific

- [ ] All clippy warnings addressed
- [ ] Code is formatted with `rustfmt`
- [ ] No `unsafe` code (unless absolutely necessary and justified)
- [ ] All public items have documentation
- [ ] Tests cover new functionality
- [ ] No panics in library code (use `Result` types)

### Integration

- [ ] Works with existing RLM components
- [ ] WASM compatibility maintained (if applicable)
- [ ] UAR integration not broken
- [ ] Docker builds successfully
- [ ] Doesn't break existing examples

## Screenshots/Examples

<!-- If applicable, add screenshots or code examples -->

## Additional Context

<!-- Any additional information that would be helpful for reviewers -->

## Reviewer Notes

<!-- Any specific areas you'd like reviewers to focus on -->

---

### For Maintainers

- [ ] Backport required (specify versions)
- [ ] Release notes entry needed
- [ ] Deployment considerations
- [ ] Monitoring/alerting implications