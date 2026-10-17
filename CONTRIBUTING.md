# Contributing

Thanks for helping. A few rules keep the project reliable.

## Before you start

- For anything larger than a small fix, open an issue first so the design can be agreed on.
- Site-specific logic does not go into the engine or the app core. It belongs in a provider (planned), which can be updated independently.
- Features that bypass DRM, paywalls, logins or anti-bot measures will not be accepted. See [docs/LEGAL.md](docs/LEGAL.md).

## Checks

Every change must pass:

```powershell
cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; pnpm check; pnpm --filter @downpipe/desktop build
```

- Tests live next to the code they test (`#[cfg(test)]` modules, `crates/*/tests`, `*.test.ts` beside the source).
- Network behaviour is tested against `tools/test-server`, never against real websites. If you need a new failure mode, add a query parameter there.
- New user-visible behaviour updates the README or the relevant page in `docs/`, and `CHANGELOG.md`.

## Style

- Comments explain why, not what.
- Code, comments, UI text and commit messages are in English.
- Rust: `rustfmt` defaults, no `unwrap()` outside tests, errors carry a `kind()` the UI can show.
- UI: colours only through the tokens in `apps/desktop/src/styles.css`; no gradients, blur or decorative shadows.
- Dependencies are added with exact versions.

## Commits

Short imperative sentences, Conventional Commits style when it fits: `feat(engine): split the largest remaining segment`.
