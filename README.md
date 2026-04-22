# flowmango-terminal

A GPU-accelerated terminal emulator component built in Rust, designed to embed inside [flowmango](https://github.com/user/flowmango) scenes as an interactive game object. It speaks raw PTY, parses ANSI/VT escape sequences, and renders colored text through the quartz font engine — all at 60fps with minimal per-frame overhead.

## Architecture

```
┌──────────────┐     raw bytes     ┌──────────────┐     ScreenLines     ┌────────────┐
│   PtyHandle  │ ─────────────────▶│  TermBuffer  │ ───────────────────▶│  text.rs   │
│  (pty.rs)    │                   │ (buffer.rs)  │                     │  rendering │
└──────────────┘                   └──────────────┘                     └────────────┘
       ▲                                  │                                   │
       │ keystrokes                       │ scrollback + grid                 │ TextSpec
       │                                  ▼                                   ▼
┌──────────────┐                   ┌──────────────┐                     ┌────────────┐
│  lib.rs      │                   │    State     │                     │   Canvas   │
│  mount()     │◀─────────────────▶│ (state.rs)   │                     │  (quartz)  │
└──────────────┘                   └──────────────┘                     └────────────┘
```

### Modules

| File | Purpose |
|---|---|
| `lib.rs` | Public API. `mount()` wires up the PTY, input handlers, scroll, resize logic, and the per-frame update loop. |
| `buffer.rs` | `TermBuffer` — the core VT parser. Processes raw PTY bytes into a grid of `ScreenLine`s with per-cell foreground/background colors. Handles CSI sequences (cursor movement, erase, scroll, SGR color), UTF-8 decoding, and line wrapping on resize. |
| `pty.rs` | `PtyHandle` — spawns a child process inside a pseudo-terminal. On Unix, reads are offloaded to a background thread via `mpsc` to stay non-blocking. |
| `state.rs` | `State` — owns the `TermBuffer`, scroll position, dirty tracking flags, and the slot cache that maps visible viewport rows to canvas game objects. |
| `text.rs` | Rendering. Converts `ScreenLine` data into `TextSpec` spans with per-run color, then flushes them to the canvas. Implements a ring-buffer slot system for O(1) scroll recycling. |
| `settings.rs` | `TermSettings` — hot-reloadable configuration: font size, colors, ANSI palette, viewport dimensions, scroll speed, scrollback limit. |
| `util.rs` | Helpers: hex color parsing, ANSI-256 color mapping, solid-color image generation, chrome rebuilding, slot management. |

## Features

- **ANSI/VT100 escape sequence parsing** — cursor movement, SGR colors (16, 256, and 24-bit), erase, scroll regions, insert/delete lines, alternate screen buffer
- **Full UTF-8 support** — including CJK double-width characters, combining marks, and emoji
- **Scrollback buffer** — configurable limit (default 5000 lines), with smooth pixel-level scrolling
- **Ring-buffer slot rendering** — only visible lines are rendered; scrolling recycles slots instead of rebuilding
- **Dirty tracking** — per-row generation counters minimize per-frame text work to only what changed
- **Resize handling** — re-wraps scrollback and grid content, debounces PTY resize signals during drag
- **Hot-reloadable settings** — colors, font size, palette, and viewport dimensions update live from source files
- **Cross-platform PTY** — Unix (background reader thread) and Windows (blocking fallback) via `portable-pty`

## Usage

```rust
use flowmango::prelude::*;
use terminal::{Terminal, TermSettings};
use quartz::Shared;

let settings = Shared::new(TermSettings::default());
let font_bytes = std::fs::read("your-font.ttf").unwrap();

let term = terminal::mount(
    &mut context,
    &mut scene,
    layer_id,
    font_bytes,
    Some(settings),
);
```

The returned `Terminal` gives you shared access to `settings` (for runtime tweaks) and `pty` (for programmatic input).

## Configuration

All fields on `TermSettings` are hot-reloadable:

| Setting | Default | Description |
|---|---|---|
| `font_size` | `16.0` | Font size in virtual pixels |
| `line_height_ratio` | `1.4` | Line height as a multiple of font size |
| `char_width_ratio` | `0.6` | Character cell width as a multiple of font size |
| `scroll_speed` | `1.0` | Scroll multiplier |
| `scrollback_limit` | `5000` | Max lines retained in scrollback |
| `view_width` / `view_height` | `0.0` | Viewport override (0 = use canvas size) |
| `offset_x` / `offset_y` | `0.0` | Top-left offset within canvas |
| `background` | `#000000` | Background color |
| `color_text` | `#e2dff0` | Default text color |
| `color_cursor` | `#9d6bff` | Cursor color |
| `color_0` — `color_15` | *(pastel dark theme)* | ANSI 16-color palette |

## Dependencies

- **flowmango** — scene/canvas framework
- **quartz** — font rendering, color types, shared state
- **portable-pty** — cross-platform pseudo-terminal
- **image** — solid-color texture generation