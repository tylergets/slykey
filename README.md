# slykey

`slykey` is a minimal global text expansion CLI for Linux X11 (tested target: i3).

It listens globally for trigger strings and replaces them with configured expansion text or macro sequences.

## Features

- Global trigger detection on X11
- YAML-based expansion config
- Expansion macros for keys and delays
- Nix flake packaging
- Home Manager module with declarative expansions

## Requirements

- Linux with X11 session
- Input simulation support for your session/environment
- For Nix usage: flakes enabled

## Install and run

### Cargo (development)

```bash
cargo run -- run
```

Validate config only:

```bash
cargo run -- validate-config
```

### Nix flake app

```bash
nix run .#default
```

### Nix dev shell

```bash
nix develop
cargo check
```

## CLI usage

```bash
slykey [OPTIONS] [COMMAND]
```

Commands:

- `run` (default when omitted)
- `validate-config`

Options:

- `-c, --config <PATH>`: explicit config path override (including Nix store paths)

Examples:

```bash
slykey run
slykey validate-config
slykey --config /path/to/config.yaml run
slykey -c /path/to/config.yaml validate-config
```

## Configuration

### Config lookup order (without `--config`)

1. `slykey.yaml` in current working directory
2. `~/.config/slykey/config.yaml`

If both exist, the CWD config takes precedence.

### Config schema

```yaml
match_behavior: immediate # immediate | boundary
boundary_chars: " \t\n.,;:!?)]}>'\"" # optional
expansions:
  - trigger: "tg@"
    expansion: "tylergetsay@gmail.com"
  - trigger: "sig;"
    expansion: "Thanks, Tyler{{KEY:ENTER}}"
```

Validation rules:

- `expansions` must not be empty
- each `trigger` must be non-empty
- `trigger` values must be unique

### Expansion macros

Supported macro forms inside `expansion`:

- `{{KEY:...}}` for key presses
- `{{SLEEP_MS:...}}` for timing pauses

Examples:

- `{{KEY:ENTER}}`
- `{{KEY:TAB}}`
- `{{KEY:ESC}}`
- `{{KEY:BACKSPACE}}`
- arrow keys, home/end, delete, page keys, `F1..F12`
- `{{SLEEP_MS:100}}`

Any non-macro text in `expansion` is typed literally.

## Home Manager module

This flake exports a module at `homeManagerModules.default`.

### Add to your Home Manager flake

```nix
{
  inputs.slykey.url = "path:/path/to/slykey";

  outputs = { nixpkgs, home-manager, slykey, ... }: {
    homeConfigurations.me = home-manager.lib.homeManagerConfiguration {
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
      modules = [
        slykey.homeManagerModules.default
        {
          programs.slykey = {
            enable = true;
            expansions = [
              {
                trigger = "tg@";
                expansion = "tylergetsay@gmail.com";
              }
              {
                trigger = "sig;";
                expansion = "Thanks, Tyler{{KEY:ENTER}}";
              }
            ];

            # Optional:
            # package = slykey.packages.x86_64-linux.default;
            matchBehavior = "immediate"; # or "boundary"
            # boundaryChars = " \t\n.,;:!?)]}>'\"";
          };
        }
      ];
    };
  };
}
```

Module behavior:

- Generates a YAML config in the Nix store from `programs.slykey.*`
- Starts a user service: `systemd.user.services.slykey`
- Runs `slykey --config /nix/store/...-slykey-config.yaml run`

## Project layout

- `src/config.rs`: config loading + validation
- `src/core/engine.rs`: trigger matching and expansion execution
- `src/core/expansion.rs`: macro parsing
- `src/io/`: input/output interfaces
- `src/platform/x11_rdev.rs`: X11 backend (`rdev` listener + `enigo` output)
- `nix/home-manager.nix`: Home Manager module

## License

Add a license section/file if you plan to publish this project.
