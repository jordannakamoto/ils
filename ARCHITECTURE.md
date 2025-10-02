# Architecture Guide

Internal architecture of `ils`.

## Overview

Terminal file browser in Rust. Uses `crossterm` for terminal control and `syntect` for syntax highlighting.

## Core Components

### 1. Main Event Loop (`run_browser`)

Standard synchronous event loop:
- Draw current state
- Wait for keyboard input
- Update state
- Repeat

```
┌─────────────────┐
│   Event Loop    │
│                 │
│  ┌───────────┐  │
│  │   Draw    │──┼──> Terminal Output
│  └───────────┘  │
│       ↓         │
│  ┌───────────┐  │
│  │ Read Event│<─┼──  User Input
│  └───────────┘  │
│       ↓         │
│  ┌───────────┐  │
│  │Update State  │
│  └───────────┘  │
│       ↑         │
│       └─────────┘
└─────────────────┘
```

### 2. FileBrowser Struct

Main state container:

```rust
struct FileBrowser {
    // Directory state
    current_dir: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,

    // Layout
    num_cols: usize,
    start_row: u16,

    // Preview
    preview_mode: bool,
    preview_scroll_map: HashMap<PathBuf, usize>,
    preview_split_ratio: f32,
    preview_cache: HashMap<PathBuf, Vec<String>>,

    // Configuration
    keybindings: Keybindings,
    show_dir_slash: bool,

    // Syntax highlighting
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}
```

### 3. Layout System

**Square Distribution Algorithm**

Prefers square-ish grids over wide horizontal layouts for readability:

```
Input: 12 items, terminal width allows 6 columns

┌─────────────────────────────────────┐
│ Naive (6x2):                        │
│ item1  item2  item3  item4  item5  item6 │
│ item7  item8  item9  item10 item11 item12│
│                                     │
│ Score: |6 - 2| = 4 (worse)         │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Square (4x3):                       │
│ item1  item2  item3  item4          │
│ item5  item6  item7  item8          │
│ item9  item10 item11 item12         │
│                                     │
│ Score: |4 - 3| = 1 (better)        │
└─────────────────────────────────────┘
```

### 4. Preview System

**Lazy Loading + Bounds Checking**

```
File (1000 lines)
┌─────────────────┐
│ line 1          │
│ line 2          │
│ ...             │
│ line 100 ◄──────┼──── scroll_pos = 100
│ line 101        │
│ ...             │  ┌──────────────────┐
│ line 120 ◄──────┼──│ Visible Preview  │
│ ...             │  │ (20 lines)       │
│ line 900        │  └──────────────────┘
│ ...             │
│ line 1000       │  max_scroll = 1000 - 20 = 980
└─────────────────┘
```

Only reads what's visible:
- `BufReader` for efficient I/O
- Skip to scroll position
- Read only `preview_lines` count
- Bounds checking prevents scrolling past end

### 5. Keybinding System

```
┌──────────────────────┐
│  Default Bindings    │
│  (hardcoded)         │
└──────────┬───────────┘
           │
           ↓
    ┌─────────────┐
    │ First Run?  │
    └──────┬──────┘
           │
    ┌──────┴──────┐
    │     Yes     │     No
    ↓             ↓
┌─────────┐   ┌─────────┐
│  Save   │   │  Load   │
│ Default │   │  Custom │
└────┬────┘   └────┬────┘
     │             │
     └──────┬──────┘
            ↓
    ~/.config/ils/
    keybindings.toml
```

### 6. Terminal Integration

```
┌─────────────┐
│  User runs  │
│     ils     │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│  ils-bin    │──────┐
│  (Rust)     │      │
└──────┬──────┘      │
       │             │
       ↓             ↓
┌─────────────┐  ┌──────────────┐
│ User selects│  │ Preview/Nav  │
│    Space    │  │   p/h/j/l    │
└──────┬──────┘  └──────────────┘
       │
       ↓
┌─────────────┐
│ Write path  │
│/tmp/ils_cd  │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│Shell wrapper│
│ reads file  │
│  & cd's     │
└─────────────┘
```

## Data Flow

### File Selection Flow

```
User Input (Space)
    ↓
Check if file or directory
    ↓
┌───────┴──────┐
│              │
File           Directory
│              │
↓              ↓
Write pwd      Write dir path
to /tmp/ils_cd to /tmp/ils_cd
↓              ↓
Open $EDITOR   Exit, shell cd's
↓
Return to ils
(at same location)
```

### Preview Rendering Flow

```
Navigation Event (arrow/wasd)
    ↓
Update selected index
    ↓
Redraw (if preview_mode)
    ↓
Get selected file
    ↓
┌──────────────┐
│ In cache?    │
└──┬───────┬───┘
   │       │
   Yes     No
   │       │
   │       ↓
   │   ┌─────────────┐
   │   │ Open file   │
   │   │ BufReader   │
   │   └──────┬──────┘
   │          │
   │          ↓
   │   ┌─────────────┐
   │   │ Detect      │
   │   │ syntax      │
   │   └──────┬──────┘
   │          │
   ↓          ↓
┌──────────────────┐
│ Highlight lines  │
│ (visible only)   │
└────────┬─────────┘
         │
         ↓
   Render to screen
```

## Performance

### Lazy Loading
- Directory entries loaded on navigation
- Preview loads visible lines only
- Syntax highlighting on-demand

### Efficient I/O
```rust
// Bad: Reads entire file
let content = fs::read_to_string(path)?;
let lines: Vec<_> = content.lines().collect();

// Good: Reads only what's needed
let file = fs::File::open(path)?;
let reader = BufReader::new(file);
let lines: Vec<_> = reader.lines()
    .skip(scroll_pos)
    .take(preview_lines)
    .collect();
```

### Rendering
```rust
// execute! flushes immediately
execute!(stdout, Clear(FromCursorDown))?;

// queue! batches operations
queue!(stdout, MoveTo(x, y))?;
queue!(stdout, Print(text))?;
stdout.flush()?;
```

### Caching
- Preview height persisted across sessions
- Per-file scroll positions cached
- Keybindings loaded once at startup

## File Structure

```
src/
└── main.rs
    ├── install()           - Installation routine
    ├── Keybindings         - Key configuration
    ├── FileBrowser         - Main state/logic
    │   ├── new()
    │   ├── load_entries()
    │   ├── update_layout() - Square algorithm
    │   ├── draw()          - Render to terminal
    │   ├── draw_help()
    │   ├── select_*()      - Navigation
    │   ├── open_selected()
    │   └── go_*()
    ├── main()              - Entry point
    └── run_browser()       - Event loop
```

## Configuration Files

```
~/.config/ils/
├── keybindings.toml    - User key mappings
└── preview_ratio       - Preview pane height (0.0-1.0)

/tmp/
└── ils_cd              - Temp file for shell communication
```

## Dependencies

- **crossterm** - Terminal manipulation (colors, cursor, events)
- **syntect** - Syntax highlighting engine
- **serde/toml** - Configuration serialization

## Potential Improvements

- Async I/O with tokio
- Background syntax highlighting
- Directory listing cache
- Incremental syntax highlighting
- Image previews (Kitty/iTerm2 protocols)

## Testing

Manual testing only. Could add:
- Unit tests for layout algorithm
- Integration tests for keybindings
- Benchmarks for preview rendering
