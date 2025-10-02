# ils

Interactive file browser for the terminal.

## Features

- File preview with syntax highlighting
- Customizable keybindings via TOML
- Navigate with arrows or WASD
- Adjustable preview pane that persists between sessions
- Opens files in `$EDITOR` or cd into directories
- Lazy loading for large directories

## Install

```bash
cargo build --release
./target/release/ils-bin --install
source ~/.zshrc  # or ~/.bashrc
```

The installer creates `~/.config/ils/` and adds a shell function to your rc file.

## Usage

```bash
ils
```

### Keybindings

**Navigation**
- `w`/`↑` - up
- `s`/`↓` - down
- `a`/`←` - left
- `d`/`→` - right
- `l`/`Enter` - open directory
- `j`/`b`/`Backspace` - go back
- `h` - go home

**Actions**
- `Space` - select (opens files in `$EDITOR`, cd into directories)
- `q`/`Esc` - quit

**Preview**
- `p` - toggle preview
- `i`/`o` - scroll preview
- `Shift+I`/`Shift+O` - scroll faster
- `-`/`+` - adjust height

## Configuration

Configuration files live in `~/.config/ils/`:

- `keybindings.toml` - customize keybindings
- `colors.toml` - path bar colors (supports hex)
- `preview_ratio` - preview height (auto-saved)

## How it works

Pressing Space writes the selected path to `/tmp/ils_cd`. The shell wrapper reads this and either cd's (directory) or returns the path (file).

## License

MIT
