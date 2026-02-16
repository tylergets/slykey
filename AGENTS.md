# Agent Instructions for `slykey`

## Environment
- Use `nix develop` to enter the project dev shell.
- `cargo` is available from within `nix develop`.

## Required Workflow
- When you create a new file, always stage it immediately with `git add <path>`.
- After making code changes, validate them by running `nix build`.

## Validation
- Treat `nix build` as the required project-level verification step before finishing work.
