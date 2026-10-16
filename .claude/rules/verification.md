# Verification and completion

- For reported behavioral bugs, add a focused regression test that exercises the failure. Follow `CONTRIBUTING.md` for Rust, Axum and GPUI test patterns; use test-owned state.
- Run the relevant checks from `README.md` through the platform-appropriate Cargo wrapper. Include the desktop `ui-tests` feature when checking GPUI interaction changes. Documentation-only work needs link/frontmatter review, not a Rust rebuild.
- Report evidence separately: formatting/static checks, automated behavior tests, native visual/interaction review, and live Google verification. A mock-provider test does not prove consent or real-account sync works.
- For UI changes, verify affected hit targets, overlay scrolling, minimum-size layout, date alignment and pending/failure behavior. Compare against the user's reference images when supplied. State clearly if native inspection is unavailable.
- Report macOS readiness as architectural preparation until the app has been built and exercised on a Mac.
- Before calling a feature complete, trace the user's flow through its required permission, fetch/write and visible outcome. If consent or another external prerequisite remains, identify the exact blocked step and the action needed; distinguish implemented from verified.
- Finish with what changed, checks actually run and material limitations. Keep existing docs aligned; avoid adding a new report for each small change.
