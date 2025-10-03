# ils

Interactive file browser for the terminal.

https://github.com/user-attachments/assets/715f8df6-77b7-42a5-8ebf-9776ef61292f



## Features

- Customizable keybindings via TOML
  defaults:
- movement = wasd
- back/fwd directory = j/l
- cd folder or open file = space
- home directory = h
- toggle hidden folders = .
- show help = ?
- File preview with syntax highlighting
- Opens files in default shell `$EDITOR`

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
? - for help

## Configuration

Configuration files live in `~/.config/ils/`:

- `keybindings.toml` - customize keybindings
- `colors.toml` - path bar colors (supports hex)
- `preview_ratio` - preview height (auto-saved)

## How it works

Pressing Space writes the selected path to `/tmp/ils_cd`. The shell wrapper reads this and either cd's (directory) or returns the path (file).

## License

MIT
