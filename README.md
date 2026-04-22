This is a minimal full-screen terminal editor for `.tlvl` levels.

Current scope:
- Uses the `tellus_level` crate from `../tellus_level` as the format implementation.
- Starts with either a blank `32x18` level or an existing `.tlvl` file path.
- Runs as a full-screen `ratatui` + `crossterm` application.
- Has a left sidebar for editor state, a canvas on the right, and a bottom command bar.
- Supports cursor movement, layer switching, zoom, normal/insert/visual modes, yank/paste, and file commands.
- Can map a folder of images to one layer and use the first 9 images as paintable tile IDs.
- Expands `~` in command paths and startup paths.
- Falls back to numeric tile display when a painted tile ID has no mapped image.
- Loads optional startup configuration from `~/.tellus-42.conf`.
- Saves back to the binary Tellus Level format.

Current keyboard flow:

```text
Normal mode:
h j k l / arrows    move cursor
J / K               switch active layer
i                   enter insert mode
v                   enter visual mode
p                   paste yanked tiles at cursor
u                   undo last edit
Ctrl-r              redo last undone edit
+ / -               zoom
:                   open command bar

Insert mode:
h j k l / arrows    move cursor
0-9                 paint tile ID, with numeric fallback if unmapped
Esc                 leave insert mode

Visual mode:
h j k l / arrows    expand or shrink rectangular selection
0-9                 paint the selected area with a tile ID
y                   yank the selected area
p                   paste yanked tiles at the selection origin
v / Esc             leave visual mode

Command bar:
:w [path]
:q
:open <path>
:new <width> <height> [path]
:map <ground|detail|logic> <folder>
:fill <0-9>
:help
```

Example:

```text
cargo run
:map ground ./assets/ground
i
1
Esc
:w ./assets/test_level.tlvl
```

Current mapping behavior:
- Images are sorted lexicographically within the folder before mapping.
- The first 9 readable images become tile IDs `1..=9`.
- Unreadable image files are skipped instead of aborting the editor.
- Empty tile `0` remains unmapped.
- `~` is expanded in `:map`, `:open`, `:w`, and startup file paths.

Current editing behavior:
- Unmapped tile IDs render as numbers inside the grid.
- `:fill <0-9>` fills the active layer with one tile ID.
- Visual mode uses an inclusive rectangular selection anchored where `v` was pressed.
- Yank and paste operate on rectangular blocks, clipped to the level bounds when pasted near edges.

Configuration:
- The editor looks for `~/.tellus-42.conf` on startup.
- The format is plain `key=value` with `#` or `;` comments.
- Supported mapping keys: `ground_images`, `detail_images`, `logic_images`.
- Supported layout keys: `sidebar_width`, `tile_gap_x`, `tile_gap_y`.
- Supported theme keys: `sidebar_bg`, `panel_border`, `panel_text`, `muted_text`, `accent_text`, `success_text`, `warning_text`, `error_text`, `grid_bg`, `tile_bg`, `cursor_normal`, `cursor_insert`, `cursor_command`.
- Theme colors use `#RRGGBB`.

Example config:

```text
ground_images=~/art/ground
detail_images=~/art/detail
logic_images=~/art/logic

sidebar_width=40
tile_gap_x=1
tile_gap_y=1

accent_text=#a9c7ff
grid_bg=#d7d7d7
cursor_normal=#496fad
cursor_insert=#5a9563
cursor_command=#a98647
```

A fuller example file lives at `examples/tellus-42.conf.example`.

This is still a first foundation. It aims to make the editor feel like an actual tool now, while keeping the data model and controls narrow enough to iterate on safely.
