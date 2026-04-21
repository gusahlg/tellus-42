This is a minimal terminal-first `.tlvl` editor built in Rust.

Current scope:
- Uses the `tellus_level` crate from `../tellus_level` as the format implementation.
- Starts with either a blank `8x6` level or an existing `.tlvl` file path.
- Accepts editor commands from a prompt.
- Can inspect and update the three dense tile layers: `ground`, `detail`, and `logic`.
- Saves back to the binary Tellus Level format.

Current commands:

```text
help
show
new <width> <height> [path]
open <path>
set <ground|detail|logic> <x> <y> <value>
save [path]
quit
```

Example:

```text
cargo run
set ground 3 4 12
set logic 3 4 1
save assets/test_level.tlvl
show
quit
```

This is intentionally a narrow foundation. The next step can either keep this command-driven model and add more editing primitives, or replace the prompt loop with a richer full-screen TUI once the editing model feels right.
