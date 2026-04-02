## Summary

<!-- Brief description of what this PR does and why -->

## Changes

<!-- Bulleted list of specific changes -->

-

## Testing

<!-- How were these changes tested? -->

- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] Manual testing done (if applicable)

## Security Checklist

<!-- If this PR touches command validation, auth, or the SSH layer -->

- [ ] No new shell metacharacters can bypass validation
- [ ] No secrets leak to stdout or logs
- [ ] Timeouts are enforced on new code paths
- [ ] Config-driven policy is not bypassable at runtime

## Related Issues

<!-- Link any related issues: Fixes #123, Closes #456 -->
