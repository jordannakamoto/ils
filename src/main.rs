use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::{
    collections::HashMap,
    env,
    fs,
    io::{self, Write},
    path::PathBuf,
    thread,
    time::Duration,
    sync::{Arc, Mutex},
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::as_24_bit_terminal_escaped,
};
use viuer::{Config as ViuerConfig, print_from_file};
use pdf_extract::extract_text;
use serde::{Deserialize, Serialize};

fn install() -> io::Result<()> {
    println!("Installing ils...\n");

    // Create config directory
    let home = env::var("HOME").map_err(|_| io::Error::new(io::ErrorKind::Other, "HOME not set"))?;
    let config_dir = PathBuf::from(&home).join(".config/ils");
    fs::create_dir_all(&config_dir)?;
    println!("✓ Created config directory: {}", config_dir.display());

    // Create default keybindings
    let keybindings = Keybindings::default();
    keybindings.save()?;
    println!("✓ Created default keybindings: {}/.config/ils/keybindings.toml", home);

    // Create default color config
    let color_config = ColorConfig::default();
    color_config.save()?;
    println!("✓ Created default color config: {}/.config/ils/colors.toml", home);

    // Create default settings
    let settings = Settings::default();
    settings.save()?;
    println!("✓ Created default settings: {}/.config/ils/settings.toml", home);

    // Create default preview ratio
    let preview_ratio_path = config_dir.join("preview_ratio");
    fs::write(&preview_ratio_path, "0.5")?;
    println!("✓ Created preview ratio config: {}", preview_ratio_path.display());

    // Detect shell and add function
    let shell_rc = if PathBuf::from(&home).join(".zshrc").exists() {
        PathBuf::from(&home).join(".zshrc")
    } else if PathBuf::from(&home).join(".bashrc").exists() {
        PathBuf::from(&home).join(".bashrc")
    } else {
        println!("\n⚠ Could not detect shell config file (.zshrc or .bashrc)");
        println!("Please manually add the following to your shell config:\n");
        print_shell_function();
        return Ok(());
    };

    // Check if already installed
    let shell_content = fs::read_to_string(&shell_rc).unwrap_or_default();
    if shell_content.contains("ils-bin") {
        println!("✓ Shell function already installed in {}", shell_rc.display());
    } else {
        // Append shell function
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&shell_rc)?;

        writeln!(file, "\n# Interactive ls (ils)")?;
        writeln!(file, "ils() {{")?;
        writeln!(file, "    ils-bin \"$@\"")?;
        writeln!(file, "    if [ -f /tmp/ils_cd ]; then")?;
        writeln!(file, "        local target=$(cat /tmp/ils_cd)")?;
        writeln!(file, "        rm /tmp/ils_cd")?;
        writeln!(file, "        if [ -d \"$target\" ]; then")?;
        writeln!(file, "            cd \"$target\"")?;
        writeln!(file, "        else")?;
        writeln!(file, "            echo \"$target\"")?;
        writeln!(file, "        fi")?;
        writeln!(file, "    fi")?;
        writeln!(file, "}}")?;

        println!("✓ Added shell function to {}", shell_rc.display());
    }

    println!("\n✨ Installation complete!");
    println!("\nRun 'source {}' or restart your shell to use 'ils'", shell_rc.display());

    Ok(())
}

fn print_shell_function() {
    println!(r#"ils() {{
    ils-bin "$@"
    if [ -f /tmp/ils_cd ]; then
        local target=$(cat /tmp/ils_cd)
        rm /tmp/ils_cd
        if [ -d "$target" ]; then
            cd "$target"
        else
            echo "$target"
        fi
    fi
}}
"#);
}

#[derive(Serialize, Deserialize, Clone)]
struct Keybindings {
    up: Vec<char>,
    down: Vec<char>,
    left: Vec<char>,
    right: Vec<char>,
    open: Vec<char>,
    back: Vec<char>,
    home: Vec<char>,
    quit: Vec<char>,
    quit_then_open_in_finder: Vec<char>,
    help: Vec<char>,
    preview_toggle: Vec<char>,
    preview_up: Vec<char>,
    preview_down: Vec<char>,
    preview_height_decrease: Vec<char>,
    preview_height_increase: Vec<char>,
    toggle_hidden: Vec<char>,
    fuzzy_find: Vec<char>,
    fuzzy_back: Vec<char>,
    fuzzy_home: Vec<char>,
    toggle_mode: Vec<char>,
    rename: Vec<char>,
    next_sibling: Vec<char>,
    prev_sibling: Vec<char>,
    copy: Vec<char>,
    paste: Vec<char>,
    trash: Vec<char>,
    delete: Vec<char>,
    undo: Vec<char>,
    redo: Vec<char>,
    create: Vec<char>,
    jump_up: Vec<char>,
    jump_down: Vec<char>,
    jump_left: Vec<char>,
    jump_right: Vec<char>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ColorConfig {
    #[serde(default = "default_path_fg")]
    path_fg: String,
    #[serde(default = "default_path_bg")]
    path_bg: String,
    #[serde(default = "default_selected_fg")]
    selected_fg: String,
    #[serde(default = "default_selected_bg")]
    selected_bg: String,
    #[serde(default = "default_directory_fg")]
    directory_fg: String,
    #[serde(default = "default_directory_bg")]
    directory_bg: String,
    #[serde(default = "default_file_fg")]
    file_fg: String,
    #[serde(default = "default_file_bg")]
    file_bg: String,
    #[serde(default = "default_preview_border_fg")]
    preview_border_fg: String,
    #[serde(default = "default_cursor_fg")]
    cursor_fg: String,
    #[serde(default = "default_cursor_bg")]
    cursor_bg: String,
    #[serde(default = "default_fuzzy_highlight_fg")]
    fuzzy_highlight_fg: String,
    #[serde(default = "default_fuzzy_highlight_bg")]
    fuzzy_highlight_bg: String,
    #[serde(default = "default_line_number_fg")]
    line_number_fg: String,
}

fn default_path_fg() -> String {
    "white".to_string()
}

fn default_path_bg() -> String {
    "#333333".to_string()
}

fn default_selected_fg() -> String {
    "none".to_string()
}

fn default_selected_bg() -> String {
    "none".to_string()
}

fn default_directory_fg() -> String {
    "cyan".to_string()
}

fn default_directory_bg() -> String {
    "none".to_string()
}

fn default_file_fg() -> String {
    "none".to_string()
}

fn default_file_bg() -> String {
    "none".to_string()
}

fn default_preview_border_fg() -> String {
    "darkgrey".to_string()
}

fn default_cursor_fg() -> String {
    "green".to_string()
}

fn default_cursor_bg() -> String {
    "none".to_string()
}

fn default_fuzzy_highlight_fg() -> String {
    "#ffff00".to_string()
}

fn default_fuzzy_highlight_bg() -> String {
    "#323232".to_string()
}

fn default_line_number_fg() -> String {
    "darkgrey".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
struct Settings {
    #[serde(default = "default_exit_after_edit")]
    exit_after_edit: bool,
    #[serde(default = "default_preview_scroll_amount")]
    preview_scroll_amount: usize,
    #[serde(default = "default_show_hidden")]
    show_hidden: bool,
    #[serde(default = "default_preview_on_start")]
    preview_on_start: bool,
    #[serde(default = "default_preview_split_ratio")]
    preview_split_ratio: f32,
    #[serde(default = "default_case_sensitive_search")]
    case_sensitive_search: bool,
    #[serde(default = "default_show_dir_slash")]
    show_dir_slash: bool,
    #[serde(default = "default_jump_amount")]
    jump_amount: usize,
    #[serde(default = "default_show_tilde_for_home")]
    show_tilde_for_home: bool,
    #[serde(default = "default_verbose_dates")]
    verbose_dates: bool,
}

fn default_exit_after_edit() -> bool {
    false
}

fn default_jump_amount() -> usize {
    5
}

fn default_preview_scroll_amount() -> usize {
    10
}

fn default_show_hidden() -> bool {
    false
}

fn default_preview_on_start() -> bool {
    false
}

fn default_show_tilde_for_home() -> bool {
    true
}

fn default_verbose_dates() -> bool {
    false
}

fn default_preview_split_ratio() -> f32 {
    0.5
}

fn default_case_sensitive_search() -> bool {
    false
}

fn default_show_dir_slash() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            exit_after_edit: default_exit_after_edit(),
            preview_scroll_amount: default_preview_scroll_amount(),
            show_hidden: default_show_hidden(),
            preview_on_start: default_preview_on_start(),
            preview_split_ratio: default_preview_split_ratio(),
            case_sensitive_search: default_case_sensitive_search(),
            show_dir_slash: default_show_dir_slash(),
            jump_amount: default_jump_amount(),
            show_tilde_for_home: default_show_tilde_for_home(),
            verbose_dates: default_verbose_dates(),
        }
    }
}

// Unified config structure
#[derive(Serialize, Deserialize, Clone)]
struct Config {
    #[serde(default)]
    keybindings: Keybindings,
    #[serde(default)]
    colors: ColorConfig,
    #[serde(default)]
    settings: Settings,
}

impl Config {
    fn load() -> (Self, Option<String>) {
        if let Ok(home) = env::var("HOME") {
            let config_path = PathBuf::from(home).join(".config/ils/config.toml");
            if let Ok(content) = fs::read_to_string(&config_path) {
                match toml::from_str(&content) {
                    Ok(config) => return (config, None),
                    Err(e) => {
                        let error_msg = format!("Config error: {} - Using defaults. Press '?' for help.", e);
                        eprintln!("\x1b[31mError loading config from {}: {}\x1b[0m", config_path.display(), e);
                        eprintln!("\x1b[33mUsing default configuration. Run 'ils config' to fix.\x1b[0m");
                        return (Config::default(), Some(error_msg));
                    }
                }
            }
        }
        (Config::default(), None)
    }

    fn path() -> Option<PathBuf> {
        if let Ok(home) = env::var("HOME") {
            Some(PathBuf::from(home).join(".config/ils/config.toml"))
        } else {
            None
        }
    }

    fn create_default() -> io::Result<()> {
        if let Some(config_path) = Config::path() {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let default_config = r##"# ILS Configuration File
# This file contains all configuration for the ils file browser

# ============================================================================
# KEYBINDINGS
# ============================================================================
[keybindings]
# Navigation
up = ['w']
down = ['s']
left = ['a']
right = ['d']

# Actions
open = ['l']                    # Open file/directory
back = ['j', 'b']              # Go back up one directory
home = ['h']                    # Go to home directory
quit = ['q']                    # Quit without cd
quit_then_open_in_finder = ['Q'] # Quit and open current directory in Finder (Shift+q)
help = ['?']                    # Show help screen

# Preview controls
preview_toggle = ['p']          # Toggle preview pane
preview_up = ['i']             # Scroll preview up
preview_down = ['o']           # Scroll preview down
preview_height_decrease = ['-', '_']
preview_height_increase = ['+', '=']

# Other
toggle_hidden = ['.']          # Toggle hidden files
fuzzy_find = ['/']             # Enter fuzzy find mode

# Fuzzy find mode controls
fuzzy_back = ['/']             # Go back one directory (in fuzzy mode)
fuzzy_home = ['?']             # Go to home directory (in fuzzy mode)

# Other
toggle_mode = ['m']            # Toggle between list and grid mode
rename = ['r']                 # Rename selected file
next_sibling = ['n']           # Go to next sibling directory
prev_sibling = ['N']           # Go to previous sibling directory (Shift+n)
copy = ['c']                   # Copy selected file to clipboard
paste = ['v']                  # Paste from clipboard
trash = ['x']                  # Move to trash
delete = ['X']                 # Permanently delete (Shift+x)
undo = ['z']                   # Undo last action
redo = ['Z']                   # Redo last undone action (Shift+z)
create = ['y']                 # Create new file or directory

# ============================================================================
# COLORS
# ============================================================================
[colors]
# Color values can be:
#   - Named colors: "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"
#   - Dark variants: "darkgrey", "darkred", etc.
#   - Hex colors: "#RRGGBB" (e.g., "#333333")
#   - Special: "reverse" (inverted default color), "none" (no color)

# Path bar at top
path_fg = "white"
path_bg = "#333333"

# Selected item
selected_fg = "none"
selected_bg = "none"

# Directory names
directory_fg = "cyan"
directory_bg = "none"

# File names
file_fg = "none"
file_bg = "none"

# Preview pane border
preview_border_fg = "darkgrey"

# Cursor (">")
cursor_fg = "green"
cursor_bg = "none"

# Fuzzy find highlighting
fuzzy_highlight_fg = "#ffff00"
fuzzy_highlight_bg = "#323232"

# ============================================================================
# SETTINGS
# ============================================================================
[settings]
# Exit after editing a file (default: false)
exit_after_edit = false

# Number of lines to scroll in preview mode (default: 10)
# Shift+i/o will scroll by visible lines instead
preview_scroll_amount = 10

# Show hidden files by default (default: false)
show_hidden = false

# Show preview pane on start (default: false)
preview_on_start = false

# Preview pane height ratio (0.0-1.0, default: 0.5)
preview_split_ratio = 0.5

# Case sensitive fuzzy find search (default: false)
case_sensitive_search = false

# Show trailing slash on directories (default: true)
show_dir_slash = true
"##;

            fs::write(&config_path, default_config)?;
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            keybindings: Keybindings::default(),
            colors: ColorConfig::default(),
            settings: Settings::default(),
        }
    }
}

impl Settings {
    fn save(&self) -> io::Result<()> {
        if let Ok(home) = env::var("HOME") {
            let config_dir = PathBuf::from(home).join(".config/ils");
            fs::create_dir_all(&config_dir)?;
            let config_path = config_dir.join("settings.toml");
            let content = toml::to_string_pretty(self).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, e)
            })?;
            fs::write(config_path, content)?;
        }
        Ok(())
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        ColorConfig {
            path_fg: default_path_fg(),
            path_bg: default_path_bg(),
            selected_fg: default_selected_fg(),
            selected_bg: default_selected_bg(),
            directory_fg: default_directory_fg(),
            directory_bg: default_directory_bg(),
            file_fg: default_file_fg(),
            file_bg: default_file_bg(),
            preview_border_fg: default_preview_border_fg(),
            cursor_fg: default_cursor_fg(),
            cursor_bg: default_cursor_bg(),
            fuzzy_highlight_fg: default_fuzzy_highlight_fg(),
            fuzzy_highlight_bg: default_fuzzy_highlight_bg(),
            line_number_fg: default_line_number_fg(),
        }
    }
}

impl ColorConfig {
    fn load() -> Self {
        if let Ok(home) = env::var("HOME") {
            let config_path = PathBuf::from(home).join(".config/ils/colors.toml");
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    fn save(&self) -> io::Result<()> {
        if let Ok(home) = env::var("HOME") {
            let config_dir = PathBuf::from(home).join(".config/ils");
            fs::create_dir_all(&config_dir)?;
            let config_path = config_dir.join("colors.toml");
            let content = toml::to_string_pretty(self).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, e)
            })?;
            fs::write(config_path, content)?;
        }
        Ok(())
    }

    fn parse_fg_color(&self) -> Option<Color> {
        Self::parse_color_string(&self.path_fg)
    }

    fn parse_bg_color(&self) -> Option<Color> {
        Self::parse_color_string(&self.path_bg)
    }

    fn parse_selected_fg(&self) -> Option<Color> {
        Self::parse_color_string(&self.selected_fg)
    }

    fn parse_selected_bg(&self) -> Option<Color> {
        Self::parse_color_string(&self.selected_bg)
    }

    fn parse_directory_fg(&self) -> Option<Color> {
        Self::parse_color_string(&self.directory_fg)
    }

    fn parse_directory_bg(&self) -> Option<Color> {
        Self::parse_color_string(&self.directory_bg)
    }

    fn parse_file_fg(&self) -> Option<Color> {
        Self::parse_color_string(&self.file_fg)
    }

    fn parse_file_bg(&self) -> Option<Color> {
        Self::parse_color_string(&self.file_bg)
    }

    fn parse_preview_border_fg(&self) -> Option<Color> {
        Self::parse_color_string(&self.preview_border_fg)
    }

    fn parse_cursor_fg(&self) -> Option<Color> {
        Self::parse_color_string(&self.cursor_fg)
    }

    fn parse_cursor_bg(&self) -> Option<Color> {
        Self::parse_color_string(&self.cursor_bg)
    }

    fn parse_fuzzy_highlight_fg(&self) -> Option<Color> {
        Self::parse_color_string(&self.fuzzy_highlight_fg)
    }

    fn parse_fuzzy_highlight_bg(&self) -> Option<Color> {
        Self::parse_color_string(&self.fuzzy_highlight_bg)
    }

    fn parse_line_number_fg(&self) -> Option<Color> {
        Self::parse_color_string(&self.line_number_fg)
    }

    fn parse_color_string(color_str: &str) -> Option<Color> {
        let color_str = color_str.trim().to_lowercase();

        // Check for hex color (#RRGGBB or #RGB)
        if color_str.starts_with('#') {
            return Self::parse_hex_color(&color_str);
        }

        // Named colors
        match color_str.as_str() {
            "black" => Some(Color::Black),
            "red" => Some(Color::Red),
            "green" => Some(Color::Green),
            "yellow" => Some(Color::Yellow),
            "blue" => Some(Color::Blue),
            "magenta" => Some(Color::Magenta),
            "cyan" => Some(Color::Cyan),
            "white" => Some(Color::White),
            "darkgrey" | "darkgray" => Some(Color::DarkGrey),
            "darkred" => Some(Color::DarkRed),
            "darkgreen" => Some(Color::DarkGreen),
            "darkyellow" => Some(Color::DarkYellow),
            "darkblue" => Some(Color::DarkBlue),
            "darkmagenta" => Some(Color::DarkMagenta),
            "darkcyan" => Some(Color::DarkCyan),
            "grey" | "gray" => Some(Color::Grey),
            "none" | "reverse" => None,
            _ => None,
        }
    }

    fn parse_hex_color(hex: &str) -> Option<Color> {
        let hex = hex.trim_start_matches('#');

        let (r, g, b) = if hex.len() == 6 {
            // #RRGGBB format
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b)
        } else if hex.len() == 3 {
            // #RGB format - expand to #RRGGBB
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            (r * 17, g * 17, b * 17) // 0xF -> 0xFF
        } else {
            return None;
        };

        Some(Color::Rgb { r, g, b })
    }
}

impl Default for Keybindings {
    fn default() -> Self {
        Keybindings {
            up: vec!['w'],
            down: vec!['s'],
            left: vec!['a'],
            right: vec!['d'],
            open: vec!['l'],
            back: vec!['j', 'b'],
            home: vec!['h'],
            quit: vec!['q'],
            quit_then_open_in_finder: vec!['Q'],
            help: vec!['?'],
            preview_toggle: vec!['p'],
            preview_up: vec!['i'],
            preview_down: vec!['o'],
            preview_height_decrease: vec!['-', '_'],
            preview_height_increase: vec!['+', '='],
            toggle_hidden: vec!['.'],
            fuzzy_find: vec!['/'],
            fuzzy_back: vec!['/'],
            fuzzy_home: vec!['?'],
            toggle_mode: vec!['m'],
            rename: vec!['r'],
            next_sibling: vec!['n'],
            prev_sibling: vec!['N'],
            copy: vec!['c'],
            paste: vec!['v'],
            trash: vec!['x'],
            delete: vec!['X'],
            undo: vec!['z'],
            redo: vec!['Z'],
            create: vec!['y'],
            jump_up: vec!['W'],
            jump_down: vec!['S'],
            jump_left: vec!['A'],
            jump_right: vec!['D'],
        }
    }
}

impl Keybindings {
    fn load() -> Self {
        if let Ok(home) = env::var("HOME") {
            let config_path = PathBuf::from(home).join(".config/ils/keybindings.toml");
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(bindings) = toml::from_str(&content) {
                    return bindings;
                }
            }
        }
        Self::default()
    }

    fn save(&self) -> io::Result<()> {
        if let Ok(home) = env::var("HOME") {
            let config_dir = PathBuf::from(home).join(".config/ils");
            fs::create_dir_all(&config_dir)?;
            let config_path = config_dir.join("keybindings.toml");
            let content = toml::to_string_pretty(self).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, e)
            })?;
            fs::write(config_path, content)?;
        }
        Ok(())
    }

    fn contains(&self, key_list: &[char], ch: char) -> bool {
        key_list.contains(&ch)
    }
}

// Action for undo/redo
#[derive(Clone)]
enum UndoAction {
    Copy { src: PathBuf, dest: PathBuf },
    Move { src: PathBuf, dest: PathBuf },
    Delete { path: PathBuf, was_dir: bool },
    Rename { old_path: PathBuf, new_path: PathBuf },
    Create { path: PathBuf, was_dir: bool },
}

#[derive(Clone)]
enum PreviewState {
    NotLoaded,
    Loading,
    Loaded(Vec<String>),
    Error(String),
}

struct FileBrowser {
    current_dir: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
    scroll_offset: usize,
    num_cols: usize,
    start_row: u16, // The row where the content starts drawing
    breadcrumbs: Vec<String>, // Track folders we've navigated into
    show_dir_slash: bool, // Whether to show trailing slash for directories
    preview_mode: bool, // Whether preview pane is active
    preview_scroll_map: HashMap<PathBuf, usize>, // Per-file scroll positions
    preview_split_ratio: f32, // Ratio of screen for preview (0.0-1.0)
    show_help: bool, // Whether to show help screen
    show_hidden: bool, // Whether to show hidden files
    fuzzy_mode: bool, // Whether fuzzy find mode is active
    fuzzy_query: String, // Current fuzzy search query
    fuzzy_prev_count: usize, // Previous match count for fuzzy finder
    fuzzy_jump_mode: bool, // Whether fuzzy mode should auto-exit on selection
    list_mode: bool, // Whether to show in list mode (vs grid mode)
    list_info_mode: u8, // 0 = none, 1 = modified date, 2 = permissions, 3 = size
    show_line_numbers: bool, // Whether to show line numbers in preview
    clipboard: Option<PathBuf>, // Copied file/directory path
    undo_stack: Vec<UndoAction>, // Undo history
    redo_stack: Vec<UndoAction>, // Redo history
    keybindings: Keybindings,
    color_config: ColorConfig,
    settings: Settings,
    preview_cache: Arc<Mutex<HashMap<PathBuf, PreviewState>>>, // Cache preview content with loading state
    syntax_set: Option<SyntaxSet>,  // Lazy-loaded on first preview
    theme_set: Option<ThemeSet>,    // Lazy-loaded on first preview
    config_error: Option<String>,   // Config loading error message
    dir_size_cache: HashMap<PathBuf, u64>, // Cache directory sizes
    calculating_sizes: bool, // Whether we're currently calculating sizes
    show_created_date: bool, // Toggle between modified and created date
    error_message: Option<String>, // Error message to display
}

impl FileBrowser {
    fn format_path_display(&self) -> String {
        if self.settings.show_tilde_for_home {
            if let Some(home) = env::var("HOME").ok() {
                let home_path = PathBuf::from(home);
                if let Ok(relative) = self.current_dir.strip_prefix(&home_path) {
                    if relative.as_os_str().is_empty() {
                        return "~".to_string();
                    } else {
                        return format!("~/{}", relative.display());
                    }
                }
            }
        }
        self.current_dir.display().to_string()
    }

    fn new(start_dir: PathBuf) -> io::Result<Self> {
        let (_, row) = cursor::position()?;

        // Load unified config or create default if not exists
        let (config, config_error) = if let Some(config_path) = Config::path() {
            if config_path.exists() {
                Config::load()
            } else {
                let _ = Config::create_default();
                (Config::default(), None)
            }
        } else {
            (Config::default(), None)
        };

        // Check if this is first run (show help if no config exists)
        let show_help = Config::path().map(|p| !p.exists()).unwrap_or(true);

        let keybindings = config.keybindings;
        let color_config = config.colors;
        let settings = config.settings;

        // Load saved preview split ratio (use saved value if exists, otherwise use config)
        let preview_split_ratio = Self::load_preview_ratio().unwrap_or(settings.preview_split_ratio);

        // start drawing content on the row *after* the initial position
        let mut browser = FileBrowser {
            current_dir: start_dir,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            num_cols: 1,
            start_row: row,
            breadcrumbs: Vec::new(),
            show_dir_slash: settings.show_dir_slash,
            preview_mode: settings.preview_on_start,
            preview_scroll_map: HashMap::new(),
            preview_split_ratio,
            show_help,
            show_hidden: settings.show_hidden,
            fuzzy_mode: false,
            fuzzy_query: String::new(),
            fuzzy_prev_count: 0,
            fuzzy_jump_mode: false,
            list_mode: false,
            list_info_mode: 0,
            show_line_numbers: true,
            clipboard: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            keybindings,
            color_config,
            settings,
            preview_cache: Arc::new(Mutex::new(HashMap::new())),
            syntax_set: None,  // Lazy-loaded
            theme_set: None,   // Lazy-loaded
            config_error,
            dir_size_cache: HashMap::new(),
            calculating_sizes: false,
            show_created_date: false,
            error_message: None,
        };
        browser.load_entries()?;
        // Don't calculate layout here - will be done on first draw for faster startup

        Ok(browser)
    }

    fn ensure_syntax_loaded(&mut self) {
        if self.syntax_set.is_none() {
            self.syntax_set = Some(SyntaxSet::load_defaults_newlines());
            self.theme_set = Some(ThemeSet::load_defaults());
        }
    }

    fn calculate_dir_size(dir: &PathBuf) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        total += metadata.len();
                    } else if metadata.is_dir() {
                        total += Self::calculate_dir_size(&entry.path());
                    }
                }
            }
        }
        total
    }

    fn calculate_all_dir_sizes(&mut self) -> io::Result<()> {
        self.calculating_sizes = true;
        for entry in &self.entries {
            if entry.is_dir() && !self.dir_size_cache.contains_key(entry) {
                let size = Self::calculate_dir_size(entry);
                self.dir_size_cache.insert(entry.clone(), size);
            }
        }
        self.calculating_sizes = false;
        Ok(())
    }

    fn start_preview_load(&self, path: PathBuf) {
        let cache = Arc::clone(&self.preview_cache);
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        // Mark as loading
        if let Ok(mut cache_lock) = cache.lock() {
            cache_lock.insert(path.clone(), PreviewState::Loading);
        }

        thread::spawn(move || {
            let result = if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp") {
                // For images, we can't really cache rendered output easily
                // Just mark as loaded with placeholder
                PreviewState::Loaded(vec!["[Image Preview]".to_string()])
            } else if extension == "pdf" {
                // Extract PDF text
                match extract_text(&path) {
                    Ok(text) => {
                        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
                        PreviewState::Loaded(lines)
                    }
                    Err(_) => PreviewState::Error("Cannot extract PDF text".to_string())
                }
            } else {
                PreviewState::NotLoaded
            };

            if let Ok(mut cache_lock) = cache.lock() {
                cache_lock.insert(path, result);
            }
        });
    }

    fn config_exists() -> bool {
        if let Ok(home) = env::var("HOME") {
            let config_path = PathBuf::from(home).join(".config/ils/preview_ratio");
            config_path.exists()
        } else {
            false
        }
    }

    fn color_config_exists() -> bool {
        if let Ok(home) = env::var("HOME") {
            let config_path = PathBuf::from(home).join(".config/ils/colors.toml");
            config_path.exists()
        } else {
            false
        }
    }

    fn load_preview_ratio() -> Option<f32> {
        let home = env::var("HOME").ok()?;
        let config_path = PathBuf::from(home).join(".config/ils/preview_ratio");
        let content = fs::read_to_string(config_path).ok()?;
        content.trim().parse().ok()
    }

    fn save_preview_ratio(&self) -> io::Result<()> {
        if let Ok(home) = env::var("HOME") {
            let config_dir = PathBuf::from(home).join(".config/ils");
            fs::create_dir_all(&config_dir)?;
            let config_path = config_dir.join("preview_ratio");
            fs::write(config_path, self.preview_split_ratio.to_string())?;
        }
        Ok(())
    }

    fn save_show_hidden(&self) -> io::Result<()> {
        if let Some(config_path) = Config::path() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(mut config) = toml::from_str::<Config>(&content) {
                    config.settings.show_hidden = self.show_hidden;
                    if let Ok(new_content) = toml::to_string_pretty(&config) {
                        fs::write(&config_path, new_content)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn load_entries(&mut self) -> io::Result<()> {
        self.entries.clear();
        self.selected = 0;
        self.scroll_offset = 0;

        let mut entries: Vec<PathBuf> = fs::read_dir(&self.current_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();

        // Filter out hidden files (starting with '.') if show_hidden is false
        if !self.show_hidden {
            entries.retain(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| !n.starts_with('.'))
                    .unwrap_or(true)
            });
        }

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();
            if a_is_dir == b_is_dir {
                a.file_name().cmp(&b.file_name())
            } else {
                b_is_dir.cmp(&a_is_dir)
            }
        });

        self.entries = entries;
        self.update_layout()?; // Recalculate layout after loading new directory entries
        Ok(())
    }

    /// Recalculates the number of columns and adjusts selected/scroll indices based on current terminal size.
    fn update_layout(&mut self) -> io::Result<()> {
        let (width, height) = terminal::size()?;
        let term_width = width as usize;
        let term_height = height as usize;

        // Fixed cell width: 20 characters for name + 2 for prefix
        const CELL_WIDTH: usize = 22;

        // Calculate available rows for content (subtract header and footer)
        let available_rows = term_height.saturating_sub(self.start_row as usize).saturating_sub(2);

        // Layout method: Square
        // Distribute into a square-like layout (cols ≈ rows)
        let num_entries = self.entries.len().max(1);

        // In list mode, always use 1 column
        if self.list_mode {
            self.num_cols = 1;
        } else {
            // Calculate max columns that can fit
            let max_cols = (term_width / CELL_WIDTH).max(1);

            // Start with square root as ideal column count
            let ideal_cols = (num_entries as f64).sqrt().ceil() as usize;
            let ideal_cols = ideal_cols.clamp(1, max_cols);

            let mut best_cols = ideal_cols;
            let mut best_score = usize::MAX;

            // Try column counts around the ideal, find the one closest to square that fits
            for cols in 1..=max_cols {
                let rows_needed = (num_entries + cols - 1) / cols;
                if rows_needed <= available_rows {
                    // Score based on how close to square (minimize abs difference between cols and rows)
                    let score = if cols > rows_needed {
                        cols - rows_needed
                    } else {
                        rows_needed - cols
                    };

                    if score < best_score {
                        best_score = score;
                        best_cols = cols;
                    }
                }
            }

            self.num_cols = best_cols;
        }

        // Ensure selected index is valid after layout change
        if !self.entries.is_empty() {
            self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        }

        Ok(())
    }

    fn draw(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();

        let (width, height) = terminal::size()?;

        // Ensure layout is calculated (deferred from new() for faster startup)
        self.update_layout()?;

        // Show help screen if requested
        if self.show_help {
            self.draw_help(&mut stdout, width, height)?;
            stdout.flush()?;
            return Ok(());
        }

        // Calculate split if in preview mode
        let split_line = if self.preview_mode {
            self.start_row + ((height - self.start_row) as f32 * (1.0 - self.preview_split_ratio)) as u16
        } else {
            height
        };
        let display_height = split_line;

        // 1. Clear from start row downward (execute for immediate effect to reduce flicker)
        execute!(stdout, cursor::MoveTo(0, self.start_row))?;
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;

        // Display directory path with color config
        queue!(stdout, cursor::MoveTo(0, self.start_row))?;

        let fg_color = self.color_config.parse_fg_color();
        let bg_color = self.color_config.parse_bg_color();

        let display_path = self.format_path_display();

        if fg_color.is_none() && bg_color.is_none() {
            // Use reverse attribute (default)
            queue!(
                stdout,
                crossterm::style::SetAttribute(crossterm::style::Attribute::Reverse),
                Print(format!(" {} ", display_path)),
                crossterm::style::SetAttribute(crossterm::style::Attribute::Reset)
            )?;
        } else {
            // Use specified colors
            if let Some(fg) = fg_color {
                queue!(stdout, SetForegroundColor(fg))?;
            }
            if let Some(bg) = bg_color {
                queue!(stdout, crossterm::style::SetBackgroundColor(bg))?;
            }
            queue!(stdout, Print(format!(" {} ", display_path)))?;
            queue!(stdout, ResetColor)?;
        }

        // Explicitly move to next line
        queue!(stdout, cursor::MoveTo(0, self.start_row + 1))?;

        // Display breadcrumbs (currently hidden)
        // for folder in &self.breadcrumbs {
        //     queue!(
        //         stdout,
        //         SetForegroundColor(Color::DarkGrey),
        //         Print(format!("← {}/", folder)),
        //         ResetColor,
        //         Print("\n")
        //     )?;
        // }

        let start_content_row = self.start_row + 1; // + self.breadcrumbs.len() as u16;
        // Move cursor to where file list starts.
        queue!(stdout, cursor::MoveTo(0, start_content_row))?;

        // Display entries
        if self.entries.is_empty() {
            queue!(
                stdout,
                SetForegroundColor(Color::Yellow),
                Print("  (empty directory)\n"),
                ResetColor
            )?;
        } else {
            // Fixed dimensions for grid
            const CELL_WIDTH: usize = 22;
            const NAME_WIDTH: usize = 20;

            let max_display_rows = (display_height as usize).saturating_sub(self.start_row as usize).saturating_sub(2); // + self.breadcrumbs.len());
            let total_rows = (self.entries.len() + self.num_cols - 1) / self.num_cols;

            // Use scroll_offset to show the right portion (works for both list and grid mode)
            let start_row = self.scroll_offset;
            let end_row = (start_row + max_display_rows).min(total_rows);
            let num_rows = end_row - start_row;

            for row in start_row..end_row {
                for col in 0..self.num_cols {
                    let idx = row * self.num_cols + col;

                    if idx >= self.entries.len() {
                        // Don't print padding for empty cells, just break
                        break;
                    }

                    let entry = &self.entries[idx];
                    let is_selected = idx == self.selected;
                    let is_dir = entry.is_dir();

                    let name = entry.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?");

                    // Truncate name if needed
                    let mut display_name = if is_dir && self.show_dir_slash {
                        format!("{}/", name)
                    } else {
                        name.to_string()
                    };

                    // In grid mode or list mode with info, truncate to NAME_WIDTH
                    // In list mode without info, don't truncate
                    if (!self.list_mode || (self.list_mode && self.list_info_mode > 0)) && display_name.len() > NAME_WIDTH {
                        display_name.truncate(NAME_WIDTH - 1);
                        display_name.push('~');
                    }

                    let prefix = if is_selected { "> " } else { "  " };

                    // Check if this entry matches the fuzzy query
                    let query_len = if self.fuzzy_mode && !self.fuzzy_query.is_empty() {
                        let (query_cmp, name_cmp) = if self.settings.case_sensitive_search {
                            (self.fuzzy_query.clone(), name.to_string())
                        } else {
                            (self.fuzzy_query.to_lowercase(), name.to_lowercase())
                        };
                        if name_cmp.starts_with(&query_cmp) {
                            self.fuzzy_query.len().min(display_name.len())
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    // Print prefix with cursor color
                    if is_selected {
                        // Apply cursor colors for the ">" prefix
                        if let Some(cursor_color) = self.color_config.parse_cursor_fg() {
                            queue!(stdout, SetForegroundColor(cursor_color))?;
                        } else {
                            queue!(stdout, SetForegroundColor(Color::Green))?;
                        }
                        if let Some(cursor_bg) = self.color_config.parse_cursor_bg() {
                            queue!(stdout, crossterm::style::SetBackgroundColor(cursor_bg))?;
                        }
                    }
                    queue!(stdout, Print(prefix))?;

                    // Now set the colors for the filename
                    if is_selected {
                        // Apply selected colors
                        if let Some(fg) = self.color_config.parse_selected_fg() {
                            queue!(stdout, SetForegroundColor(fg))?;
                        } else {
                            queue!(stdout, SetForegroundColor(Color::Green))?;
                        }
                        if let Some(bg) = self.color_config.parse_selected_bg() {
                            queue!(stdout, crossterm::style::SetBackgroundColor(bg))?;
                        }
                    } else if is_dir {
                        // Apply directory colors
                        if let Some(fg) = self.color_config.parse_directory_fg() {
                            queue!(stdout, SetForegroundColor(fg))?;
                        } else {
                            queue!(stdout, SetForegroundColor(Color::Blue))?;
                        }
                        if let Some(bg) = self.color_config.parse_directory_bg() {
                            queue!(stdout, crossterm::style::SetBackgroundColor(bg))?;
                        }
                    } else {
                        // Apply file colors
                        if let Some(fg) = self.color_config.parse_file_fg() {
                            queue!(stdout, SetForegroundColor(fg))?;
                        } else {
                            queue!(stdout, ResetColor)?;
                        }
                        if let Some(bg) = self.color_config.parse_file_bg() {
                            queue!(stdout, crossterm::style::SetBackgroundColor(bg))?;
                        }
                    }

                    // Print name with fuzzy match highlighting
                    if query_len > 0 {
                        let highlight_len = query_len.min(display_name.len());

                        // Print matching part with fuzzy highlight colors
                        queue!(stdout, crossterm::style::SetAttribute(crossterm::style::Attribute::Bold))?;
                        if let Some(fg) = self.color_config.parse_fuzzy_highlight_fg() {
                            queue!(stdout, SetForegroundColor(fg))?;
                        } else {
                            queue!(stdout, SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 0 }))?;
                        }
                        if let Some(bg) = self.color_config.parse_fuzzy_highlight_bg() {
                            queue!(stdout, crossterm::style::SetBackgroundColor(bg))?;
                        } else {
                            queue!(stdout, crossterm::style::SetBackgroundColor(Color::Rgb { r: 50, g: 50, b: 50 }))?;
                        }
                        queue!(stdout, Print(&display_name[..highlight_len]))?;
                        queue!(stdout, crossterm::style::SetAttribute(crossterm::style::Attribute::Reset))?;

                        // Reset to original color for rest
                        if is_selected {
                            if let Some(fg) = self.color_config.parse_selected_fg() {
                                queue!(stdout, SetForegroundColor(fg))?;
                            } else {
                                queue!(stdout, SetForegroundColor(Color::Green))?;
                            }
                            if let Some(bg) = self.color_config.parse_selected_bg() {
                                queue!(stdout, crossterm::style::SetBackgroundColor(bg))?;
                            }
                        } else if is_dir {
                            if let Some(fg) = self.color_config.parse_directory_fg() {
                                queue!(stdout, SetForegroundColor(fg))?;
                            } else {
                                queue!(stdout, SetForegroundColor(Color::Blue))?;
                            }
                        } else {
                            queue!(stdout, ResetColor)?;
                        }

                        // Print rest of name, padded
                        let rest = &display_name[highlight_len..];
                        let padding = NAME_WIDTH - display_name.len();
                        queue!(stdout, Print(format!("{}{}", rest, " ".repeat(padding))))?;
                    } else {
                        // No match, print normally with padding
                        queue!(stdout, Print(format!("{:<width$}", display_name, width = NAME_WIDTH)))?;
                    }

                    queue!(stdout, ResetColor)?;

                    // In list mode, show info after the name
                    if self.list_mode && self.list_info_mode > 0 {
                        if self.list_info_mode == 1 {
                            // Show date (modified or created based on toggle)
                            if let Ok(metadata) = entry.metadata() {
                                let time_result = if self.show_created_date {
                                    metadata.created()
                                } else {
                                    metadata.modified()
                                };

                                if let Ok(time) = time_result {
                                    use std::time::SystemTime;
                                    if let Ok(elapsed) = SystemTime::now().duration_since(time) {
                                        let secs = elapsed.as_secs();
                                        let three_months = 3 * 30 * 24 * 60 * 60; // ~3 months in seconds

                                        let date_str = if self.settings.verbose_dates || secs < three_months {
                                            // Use relative format
                                            if secs < 60 {
                                                format!("{:>9}s", secs)
                                            } else if secs < 3600 {
                                                format!("{:>8}min", secs / 60)
                                            } else if secs < 86400 {
                                                format!("{:>8}hrs", secs / 3600)
                                            } else if secs < 2592000 { // 30 days
                                                format!("{:>7}days", secs / 86400)
                                            } else if secs < 31536000 { // 365 days
                                                format!("{:>6}month", secs / 2592000)
                                            } else {
                                                format!("{:>7}year", secs / 31536000)
                                            }
                                        } else {
                                            // Use "Aug '25" format for dates > 3 months
                                            use std::time::UNIX_EPOCH;
                                            if let Ok(duration) = time.duration_since(UNIX_EPOCH) {
                                                let timestamp = duration.as_secs() as i64;
                                                let days_since_epoch = timestamp / 86400;
                                                let year = 1970 + (days_since_epoch / 365);
                                                let day_in_year = days_since_epoch % 365;
                                                let month_idx = (day_in_year / 30).min(11) as usize;
                                                let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                                                             "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                                                format!("{} '{:02}", months[month_idx], year % 100)
                                            } else {
                                                format!("{:>9}s", secs)
                                            }
                                        };

                                        queue!(
                                            stdout,
                                            SetForegroundColor(Color::DarkGrey),
                                            Print(format!("  {}", date_str)),
                                            ResetColor
                                        )?;
                                    }
                                }
                            }
                        } else if self.list_info_mode == 2 {
                            // Show permissions
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                if let Ok(metadata) = entry.metadata() {
                                    let mode = metadata.permissions().mode();
                                    let perms = format!(
                                        "{}{}{}{}{}{}{}{}{}",
                                        if mode & 0o400 != 0 { 'r' } else { '-' },
                                        if mode & 0o200 != 0 { 'w' } else { '-' },
                                        if mode & 0o100 != 0 { 'x' } else { '-' },
                                        if mode & 0o040 != 0 { 'r' } else { '-' },
                                        if mode & 0o020 != 0 { 'w' } else { '-' },
                                        if mode & 0o010 != 0 { 'x' } else { '-' },
                                        if mode & 0o004 != 0 { 'r' } else { '-' },
                                        if mode & 0o002 != 0 { 'w' } else { '-' },
                                        if mode & 0o001 != 0 { 'x' } else { '-' },
                                    );
                                    queue!(
                                        stdout,
                                        SetForegroundColor(Color::DarkGrey),
                                        Print(format!("  {}", perms)),
                                        ResetColor
                                    )?;
                                }
                            }
                        } else if self.list_info_mode == 3 {
                            // Show size (with cached dir size)
                            if let Ok(metadata) = entry.metadata() {
                                let size = if is_dir {
                                    // Use cached size or show loading
                                    if let Some(&dir_size) = self.dir_size_cache.get(entry) {
                                        if dir_size < 1024 {
                                            format!("{:>8} B", dir_size)
                                        } else if dir_size < 1024 * 1024 {
                                            format!("{:>7.1} K", dir_size as f64 / 1024.0)
                                        } else if dir_size < 1024 * 1024 * 1024 {
                                            format!("{:>7.1} M", dir_size as f64 / (1024.0 * 1024.0))
                                        } else {
                                            format!("{:>7.1} G", dir_size as f64 / (1024.0 * 1024.0 * 1024.0))
                                        }
                                    } else if self.calculating_sizes {
                                        String::from("  calc...")
                                    } else {
                                        String::from("    <DIR>")
                                    }
                                } else {
                                    let len = metadata.len();
                                    if len < 1024 {
                                        format!("{:>8} B", len)
                                    } else if len < 1024 * 1024 {
                                        format!("{:>7.1} K", len as f64 / 1024.0)
                                    } else if len < 1024 * 1024 * 1024 {
                                        format!("{:>7.1} M", len as f64 / (1024.0 * 1024.0))
                                    } else {
                                        format!("{:>7.1} G", len as f64 / (1024.0 * 1024.0 * 1024.0))
                                    }
                                };
                                queue!(
                                    stdout,
                                    SetForegroundColor(Color::DarkGrey),
                                    Print(format!("  {}", size)),
                                    ResetColor
                                )?;
                            }
                        }
                    }
                }
                queue!(stdout, Print("\r\n"))?;
            }

            // Display config error if present (below the entries)
            if let Some(error) = &self.config_error {
                queue!(stdout, cursor::MoveTo(0, start_content_row + num_rows as u16 + 1))?;
                queue!(
                    stdout,
                    SetForegroundColor(Color::Red),
                    Print(format!("⚠ {}", error)),
                    ResetColor
                )?;
            }
        }

        // 3. Draw separator and preview if in preview mode
        if self.preview_mode {
            queue!(stdout, cursor::MoveTo(0, split_line))?;

            if let Some(border_color) = self.color_config.parse_preview_border_fg() {
                queue!(stdout, SetForegroundColor(border_color))?;
            } else {
                queue!(stdout, SetForegroundColor(Color::DarkGrey))?;
            }
            queue!(stdout, Print("─".repeat(width as usize)), ResetColor)?;

            // Draw preview
            if let Some(selected) = self.get_selected_path() {
                if selected.is_dir() {
                    // Directory preview - show contents and stats
                    let preview_lines = (height - split_line - 3) as usize;

                    if let Ok(entries) = fs::read_dir(&selected) {
                        let mut dirs = 0;
                        let mut files = 0;
                        let mut total_size: u64 = 0;
                        let mut items: Vec<(String, bool)> = Vec::new();

                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            let is_dir = path.is_dir();
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("?")
                                .to_string();

                            if is_dir {
                                dirs += 1;
                                // Add recursive size for directories
                                total_size += Self::calculate_dir_size(&path);
                                items.push((name, true));
                            } else {
                                files += 1;
                                if let Ok(metadata) = entry.metadata() {
                                    total_size += metadata.len();
                                }
                                items.push((name, false));
                            }
                        }

                        // Sort: directories first, then files
                        items.sort_by(|a, b| {
                            if a.1 == b.1 {
                                a.0.cmp(&b.0)
                            } else {
                                b.1.cmp(&a.1)
                            }
                        });

                        // Display stats
                        queue!(stdout, cursor::MoveTo(0, split_line + 1))?;
                        queue!(
                            stdout,
                            SetForegroundColor(Color::Cyan),
                            Print(format!("[DIR] {} items ({} dirs, {} files)", dirs + files, dirs, files)),
                            ResetColor
                        )?;

                        // Display size
                        let size_str = if total_size < 1024 {
                            format!("{} B", total_size)
                        } else if total_size < 1024 * 1024 {
                            format!("{:.1} KB", total_size as f64 / 1024.0)
                        } else if total_size < 1024 * 1024 * 1024 {
                            format!("{:.1} MB", total_size as f64 / (1024.0 * 1024.0))
                        } else {
                            format!("{:.1} GB", total_size as f64 / (1024.0 * 1024.0 * 1024.0))
                        };

                        queue!(stdout, cursor::MoveTo(0, split_line + 2))?;
                        queue!(
                            stdout,
                            SetForegroundColor(Color::DarkGrey),
                            Print(format!("Size: {}", size_str)),
                            ResetColor
                        )?;

                        // Display first few items
                        for (i, (name, is_dir)) in items.iter().take(preview_lines.saturating_sub(3)).enumerate() {
                            queue!(stdout, cursor::MoveTo(0, split_line + 4 + i as u16))?;
                            if *is_dir {
                                queue!(
                                    stdout,
                                    SetForegroundColor(Color::Blue),
                                    Print(format!("  {}/", name)),
                                    ResetColor
                                )?;
                            } else {
                                queue!(stdout, Print(format!("  {}", name)))?;
                            }
                        }
                    }
                } else if selected.is_file() {
                    let preview_lines = (height - split_line - 3) as usize;
                    let preview_width = width as u32;
                    let preview_height = preview_lines as u32;

                    // Check file extension for special handling
                    let extension = selected.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

                    if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp") {
                        // Image preview - render directly (can't cache)
                        // Move cursor to preview area and flush before viuer renders
                        queue!(stdout, cursor::MoveTo(0, split_line + 1))?;
                        stdout.flush()?;

                        let conf = ViuerConfig {
                            transparent: true,
                            absolute_offset: false,
                            x: 0,
                            y: 0,
                            width: Some(preview_width),
                            height: Some(preview_height),
                            ..Default::default()
                        };

                        if let Err(_) = print_from_file(&selected, &conf) {
                            queue!(stdout, cursor::MoveTo(0, split_line + 1))?;
                            queue!(stdout, Print("(cannot preview image)"))?;
                        }
                    } else if extension == "pdf" {
                        // PDF preview - use cache with background loading
                        let cache_state = if let Ok(cache_lock) = self.preview_cache.lock() {
                            cache_lock.get(&selected).cloned()
                        } else {
                            None
                        };

                        match cache_state {
                            Some(PreviewState::Loaded(lines)) => {
                                let scroll_pos = self.preview_scroll_map.get(&selected).copied().unwrap_or(0);
                                let display_lines: Vec<&String> = lines.iter()
                                    .skip(scroll_pos)
                                    .take(preview_lines)
                                    .collect();

                                for (i, line) in display_lines.iter().enumerate() {
                                    queue!(stdout, cursor::MoveTo(0, split_line + 1 + i as u16))?;

                                    if self.show_line_numbers {
                                        let line_num = scroll_pos + i + 1;
                                        let line_color = self.color_config.parse_line_number_fg()
                                            .unwrap_or(Color::DarkGrey);
                                        queue!(
                                            stdout,
                                            SetForegroundColor(line_color),
                                            Print(format!("{:4} │ ", line_num)),
                                            ResetColor
                                        )?;
                                    }

                                    queue!(stdout, Print(line.as_str()))?;
                                }
                            }
                            Some(PreviewState::Loading) => {
                                queue!(stdout, cursor::MoveTo(0, split_line + 1))?;
                                queue!(stdout, Print("Loading PDF..."))?;
                            }
                            Some(PreviewState::Error(msg)) => {
                                queue!(stdout, cursor::MoveTo(0, split_line + 1))?;
                                queue!(stdout, Print(format!("({})", msg)))?;
                            }
                            None | Some(PreviewState::NotLoaded) => {
                                // Start loading in background
                                self.start_preview_load(selected.clone());
                                queue!(stdout, cursor::MoveTo(0, split_line + 1))?;
                                queue!(stdout, Print("Loading PDF..."))?;
                            }
                        }
                    } else {
                        // Text file preview with syntax highlighting
                        if let Ok(file) = fs::File::open(&selected) {
                            use io::BufRead;
                            let reader = io::BufReader::new(file);
                            let scroll_pos = self.preview_scroll_map.get(&selected).copied().unwrap_or(0);

                            // Lazy-load syntax highlighting on first use
                            self.ensure_syntax_loaded();

                            // Try to detect syntax
                            let syntax = self.syntax_set.as_ref().unwrap()
                                .find_syntax_for_file(&selected)
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| self.syntax_set.as_ref().unwrap().find_syntax_plain_text());

                            let theme = &self.theme_set.as_ref().unwrap().themes["base16-ocean.dark"];
                            let mut highlighter = HighlightLines::new(syntax, theme);

                            // Only read the lines we need
                            let lines_to_display: Vec<String> = reader
                                .lines()
                                .skip(scroll_pos)
                                .take(preview_lines)
                                .filter_map(|l| l.ok())
                                .collect();

                            for (i, line) in lines_to_display.iter().enumerate() {
                                queue!(stdout, cursor::MoveTo(0, split_line + 1 + i as u16))?;

                                // Print line number if enabled
                                if self.show_line_numbers {
                                    let line_num = scroll_pos + i + 1;
                                    let line_color = self.color_config.parse_line_number_fg()
                                        .unwrap_or(Color::DarkGrey);
                                    queue!(
                                        stdout,
                                        SetForegroundColor(line_color),
                                        Print(format!("{:4} │ ", line_num)),
                                        ResetColor
                                    )?;
                                }

                                // Highlight the line
                                let ranges = highlighter.highlight_line(line, self.syntax_set.as_ref().unwrap()).unwrap_or_default();
                                let escaped = as_24_bit_terminal_escaped(&ranges[..], false);

                                queue!(stdout, Print(escaped), ResetColor)?;
                            }
                        } else {
                            queue!(stdout, cursor::MoveTo(0, split_line + 1))?;
                            queue!(stdout, Print("(binary file or cannot read)"))?;
                        }
                    }
                }
            }
        }

        // 4. Draw footer with filename if in preview mode
        if self.preview_mode {
            if let Some(selected) = self.get_selected_path() {
                if selected.is_file() {
                    queue!(stdout, cursor::MoveTo(0, height.saturating_sub(1)))?;
                    queue!(
                        stdout,
                        ResetColor,
                        SetForegroundColor(Color::DarkGrey),
                        Print(format!("{}", selected.file_name().and_then(|n| n.to_str()).unwrap_or(""))),
                        ResetColor
                    )?;
                }
            }
        }

        // 5. Draw fuzzy search bar if in fuzzy mode
        if self.fuzzy_mode {
            queue!(stdout, cursor::MoveTo(0, height.saturating_sub(1)))?;
            queue!(
                stdout,
                ResetColor,
                SetForegroundColor(Color::Yellow),
                Print(format!("Find: {}_", self.fuzzy_query)),
                ResetColor
            )?;
        }

        // Display error message if present
        if let Some(ref error_msg) = self.error_message {
            queue!(
                stdout,
                cursor::MoveTo(0, height - 1),
                SetForegroundColor(Color::Red),
                Print(format!(" ERROR: {} ", error_msg)),
                ResetColor
            )?;
        }

        // Flush all queued commands simultaneously to minimize flicker
        stdout.flush()?;
        Ok(())
    }

    fn draw_help(&mut self, stdout: &mut io::Stdout, width: u16, height: u16) -> io::Result<()> {
        execute!(stdout, cursor::MoveTo(0, self.start_row))?;
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;

        let help_text = vec![
            "",
            "╔══════════════════════════════════════╗",
            "║      Interactive ls - Welcome!       ║",
            "╚══════════════════════════════════════╝",
            "",
            "NAVIGATION:",
            "  wasd / ↑↓←→        -  Move cursor",
            "  l / Enter / k      -  Open directory/file",
            "  j / b / Backspace  -  Go back",
            "  h                  -  Go home",
            "  n                  -  Next sibling directory",
            "  Shift+n            -  Previous sibling directory",
            "  .                  -  Toggle hidden files",
            "  m                  -  Toggle list/grid mode",
            "",
            "FILE OPERATIONS:",
            "  r                  -  Rename selected file",
            "  y                  -  Create file/dir (end with / for dir)",
            "  c                  -  Copy to clipboard",
            "  v                  -  Paste from clipboard",
            "  x                  -  Move to trash",
            "  Shift+x            -  Permanently delete (with warning)",
            "  z                  -  Undo (copy/rename/create)",
            "  Shift+z            -  Redo",
            "",
            "FUZZY FIND:",
            "  /                  -  Enter fuzzy find mode",
            "  Type to search     -  Auto-navigate on unique match",
            "  Enter              -  Open selected item (in find mode)",
            "  /                  -  Go back (in find mode)",
            "  ?                  -  Go home (in find mode)",
            "  Esc                -  Exit find mode",
            "",
            "EXIT:",
            "  Esc                -  Quit without cd",
            "  q                  -  Exit and cd to current directory",
            "  Shift+q            -  Open current directory in Finder",
            "",
            "PREVIEW:",
            "  p                  -  Toggle preview",
            "  i / o              -  Scroll preview up/down",
            "  Shift+I / Shift+O  -  Scroll preview faster",
            "  - / +              -  Decrease/increase preview height",
            "",
            "  ?                  -  Toggle this help",
            "",
            "Press any key to continue...",
        ];

        let start_row = self.start_row + 1;

        for (i, line) in help_text.iter().enumerate() {
            queue!(stdout, cursor::MoveTo(0, start_row + i as u16))?;
            queue!(stdout, SetForegroundColor(Color::Cyan))?;
            queue!(stdout, Print(line))?;
            queue!(stdout, ResetColor)?;
        }

        Ok(())
    }

    fn select_up(&mut self) {
        // Row-major: move up one row (subtract num_cols)
        if self.selected >= self.num_cols {
            self.selected -= self.num_cols;

            // Update scroll if needed (works for both list and grid mode)
            let current_row = self.selected / self.num_cols;
            if current_row < self.scroll_offset {
                self.scroll_offset = current_row;
            }
        }
    }

    fn select_down(&mut self) {
        // Row-major: move down one row (add num_cols)
        let new_idx = self.selected + self.num_cols;
        if new_idx < self.entries.len() {
            self.selected = new_idx;

            // Update scroll if needed (works for both list and grid mode)
            if let Ok((_, height)) = terminal::size() {
                let max_display_rows = (height as usize).saturating_sub(self.start_row as usize).saturating_sub(2);
                let current_row = self.selected / self.num_cols;
                if current_row >= self.scroll_offset + max_display_rows {
                    self.scroll_offset = current_row - max_display_rows + 1;
                }
            }
        }
    }

    fn select_left(&mut self) {
        // Row-major: move left in same row
        if self.selected > 0 && self.selected % self.num_cols != 0 {
            self.selected -= 1;
        }
    }

    fn select_right(&mut self) {
        // Row-major: move right in same row
        if self.selected + 1 < self.entries.len() && (self.selected + 1) % self.num_cols != 0 {
            self.selected += 1;
        }
    }

    fn jump_up(&mut self) {
        // Jump up by configured amount
        for _ in 0..self.settings.jump_amount {
            self.select_up();
        }
    }

    fn jump_down(&mut self) {
        // Jump down by configured amount
        for _ in 0..self.settings.jump_amount {
            self.select_down();
        }
    }

    fn jump_left(&mut self) {
        // Jump left by configured amount
        for _ in 0..self.settings.jump_amount {
            self.select_left();
        }
    }

    fn jump_right(&mut self) {
        // Jump right by configured amount
        for _ in 0..self.settings.jump_amount {
            self.select_right();
        }
    }

    fn fuzzy_match(&self) -> (Option<usize>, usize) {
        // Find all entries that match the fuzzy query and return (first_match, count)
        if self.fuzzy_query.is_empty() {
            return (None, 0);
        }

        let matches: Vec<usize> = self.entries.iter().enumerate()
            .filter_map(|(idx, entry)| {
                if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
                    let matches = if self.settings.case_sensitive_search {
                        name.starts_with(&self.fuzzy_query)
                    } else {
                        name.to_lowercase().starts_with(&self.fuzzy_query.to_lowercase())
                    };
                    if matches {
                        Some(idx)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        (matches.first().copied(), matches.len())
    }

    fn open_selected(&mut self) -> io::Result<bool> {
        if self.entries.is_empty() {
            return Ok(false);
        }

        let selected_path = &self.entries[self.selected];
        if selected_path.is_dir() {
            // Save the old state before trying to navigate
            let old_dir = self.current_dir.clone();
            let old_entries = self.entries.clone();
            let old_selected = self.selected;
            let old_scroll = self.scroll_offset;

            // Add the selected folder to breadcrumbs before navigating
            if let Some(folder_name) = selected_path.file_name().and_then(|n| n.to_str()) {
                self.breadcrumbs.push(folder_name.to_string());
            }
            self.current_dir = selected_path.clone();

            // Try to load entries, if it fails, restore previous state
            if let Err(e) = self.load_entries() {
                self.current_dir = old_dir;
                self.entries = old_entries;
                self.selected = old_selected;
                self.scroll_offset = old_scroll;
                self.breadcrumbs.pop();

                // Set error message based on error kind
                if e.kind() == io::ErrorKind::PermissionDenied {
                    self.error_message = Some("Permission denied. Grant Full Disk Access to your terminal in System Settings > Privacy & Security.".to_string());
                } else {
                    self.error_message = Some(format!("Cannot access: {}", e));
                }
            }
            Ok(false)
        } else {
            // For a file, we return true, signaling main to write the path and exit.
            Ok(true)
        }
    }

    fn go_back(&mut self) -> io::Result<()> {
        if let Some(parent) = self.current_dir.parent() {
            // Pop the last breadcrumb when going back
            self.breadcrumbs.pop();
            self.current_dir = parent.to_path_buf();
            self.load_entries()?;
        }
        Ok(())
    }

    fn go_home(&mut self) -> io::Result<()> {
        if let Some(home) = env::var_os("HOME") {
            self.current_dir = PathBuf::from(home);
            self.breadcrumbs.clear();
            self.load_entries()?;
        }
        Ok(())
    }

    fn get_current_dir(&self) -> &PathBuf {
        &self.current_dir
    }

    fn get_selected_path(&self) -> Option<PathBuf> {
        self.entries.get(self.selected).cloned()
    }

    fn go_to_next_sibling(&mut self) -> io::Result<()> {
        // Go up to parent, then navigate to next sibling directory
        if let Some(parent) = self.current_dir.parent() {
            let current_name = self.current_dir.file_name();

            // Read parent directory
            let mut siblings: Vec<PathBuf> = fs::read_dir(parent)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();

            // If there are no siblings or only one (current), do nothing
            if siblings.len() <= 1 {
                return Ok(());
            }

            // Sort siblings
            siblings.sort();

            // Find current directory in siblings
            if let Some(current_idx) = siblings.iter().position(|p| p.file_name() == current_name) {
                // Go to next sibling (wrap around)
                let next_idx = (current_idx + 1) % siblings.len();
                self.current_dir = siblings[next_idx].clone();
                self.breadcrumbs.pop();
                if let Some(name) = self.current_dir.file_name().and_then(|n| n.to_str()) {
                    self.breadcrumbs.push(name.to_string());
                }
                self.load_entries()?;
            }
        }
        Ok(())
    }

    fn go_to_prev_sibling(&mut self) -> io::Result<()> {
        // Go up to parent, then navigate to previous sibling directory
        if let Some(parent) = self.current_dir.parent() {
            let current_name = self.current_dir.file_name();

            // Read parent directory
            let mut siblings: Vec<PathBuf> = fs::read_dir(parent)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();

            // If there are no siblings or only one (current), do nothing
            if siblings.len() <= 1 {
                return Ok(());
            }

            // Sort siblings
            siblings.sort();

            // Find current directory in siblings
            if let Some(current_idx) = siblings.iter().position(|p| p.file_name() == current_name) {
                // Go to previous sibling (wrap around)
                let prev_idx = if current_idx == 0 {
                    siblings.len() - 1
                } else {
                    current_idx - 1
                };
                self.current_dir = siblings[prev_idx].clone();
                self.breadcrumbs.pop();
                if let Some(name) = self.current_dir.file_name().and_then(|n| n.to_str()) {
                    self.breadcrumbs.push(name.to_string());
                }
                self.load_entries()?;
            }
        }
        Ok(())
    }

    fn copy_to_clipboard(&mut self) {
        if let Some(path) = self.get_selected_path() {
            self.clipboard = Some(path);
        }
    }

    fn paste_from_clipboard(&mut self) -> io::Result<()> {
        if let Some(src) = &self.clipboard {
            if !src.exists() {
                return Ok(()); // Source no longer exists
            }

            let file_name = src.file_name().unwrap();
            let mut dest = self.current_dir.join(file_name);

            // Handle name conflicts
            let mut counter = 1;
            while dest.exists() {
                let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("");
                let new_name = if ext.is_empty() {
                    format!("{} ({})", stem, counter)
                } else {
                    format!("{} ({}).{}", stem, counter, ext)
                };
                dest = self.current_dir.join(new_name);
                counter += 1;
            }

            // Copy file or directory recursively
            if src.is_dir() {
                self.copy_dir_recursive(src, &dest)?;
            } else {
                fs::copy(src, &dest)?;
            }

            self.undo_stack.push(UndoAction::Copy {
                src: src.clone(),
                dest: dest.clone()
            });
            self.redo_stack.clear();
            self.load_entries()?;
        }
        Ok(())
    }

    fn copy_dir_recursive(&self, src: &PathBuf, dest: &PathBuf) -> io::Result<()> {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir_recursive(&src_path, &dest_path)?;
            } else {
                fs::copy(&src_path, &dest_path)?;
            }
        }
        Ok(())
    }

    fn move_to_trash(&mut self) -> io::Result<()> {
        if let Some(path) = self.get_selected_path() {
            let old_selected = self.selected;

            // Use macOS trash command (stderr redirected to suppress sound)
            let output = std::process::Command::new("osascript")
                .arg("-e")
                .arg(format!("tell application \"Finder\" to delete POSIX file \"{}\"", path.display()))
                .stderr(std::process::Stdio::null())
                .output()?;

            if output.status.success() {
                // Don't add to undo stack - can't reliably restore from trash
                self.load_entries()?;

                // Keep selection on same index, or previous if at end
                if old_selected >= self.entries.len() && old_selected > 0 {
                    self.selected = old_selected - 1;
                } else if old_selected < self.entries.len() {
                    self.selected = old_selected;
                }
            }
        }
        Ok(())
    }

    fn delete_permanent(&mut self) -> io::Result<()> {
        if let Some(path) = self.get_selected_path() {
            let old_selected = self.selected;

            // Disable raw mode to show confirmation
            terminal::disable_raw_mode()?;
            execute!(io::stdout(), cursor::Show)?;

            print!("\nPermanently delete '{}'? This cannot be undone! (y/N): ",
                path.file_name().unwrap().to_str().unwrap());
            io::stdout().flush()?;

            let mut response = String::new();
            io::stdin().read_line(&mut response)?;

            // Re-enable raw mode
            terminal::enable_raw_mode()?;
            execute!(io::stdout(), cursor::Hide)?;

            if response.trim().to_lowercase() == "y" {
                let was_dir = path.is_dir();
                if was_dir {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }

                // Don't add to undo stack - can't restore deleted files
                self.load_entries()?;

                // Keep selection on same index, or previous if at end
                if old_selected >= self.entries.len() && old_selected > 0 {
                    self.selected = old_selected - 1;
                } else if old_selected < self.entries.len() {
                    self.selected = old_selected;
                }
            }
        }
        Ok(())
    }

    fn undo(&mut self) -> io::Result<()> {
        if let Some(action) = self.undo_stack.pop() {
            match &action {
                UndoAction::Copy { dest, .. } => {
                    // Undo copy: delete the destination
                    if dest.is_dir() {
                        fs::remove_dir_all(dest)?;
                    } else {
                        fs::remove_file(dest)?;
                    }
                    self.redo_stack.push(action);
                }
                UndoAction::Rename { old_path, new_path } => {
                    // Undo rename: rename back to old name
                    if new_path.exists() {
                        fs::rename(new_path, old_path)?;
                        self.redo_stack.push(action);
                    }
                }
                UndoAction::Create { path, was_dir } => {
                    // Undo create: delete the created file/directory
                    if path.exists() {
                        if *was_dir {
                            fs::remove_dir_all(path)?;
                        } else {
                            fs::remove_file(path)?;
                        }
                        self.redo_stack.push(action);
                    }
                }
                UndoAction::Move { .. } | UndoAction::Delete { .. } => {
                    // These shouldn't be in the stack, but if they are, ignore them
                }
            }
            self.load_entries()?;
        }
        Ok(())
    }

    fn redo(&mut self) -> io::Result<()> {
        if let Some(action) = self.redo_stack.pop() {
            match &action {
                UndoAction::Copy { src, dest } => {
                    // Redo copy
                    if src.is_dir() {
                        self.copy_dir_recursive(src, dest)?;
                    } else {
                        fs::copy(src, dest)?;
                    }
                    self.undo_stack.push(action);
                }
                UndoAction::Rename { old_path, new_path } => {
                    // Redo rename: rename to new name
                    if old_path.exists() {
                        fs::rename(old_path, new_path)?;
                        self.undo_stack.push(action);
                    }
                }
                UndoAction::Create { path, was_dir } => {
                    // Redo create: recreate the file/directory
                    if *was_dir {
                        fs::create_dir_all(path)?;
                    } else {
                        if let Some(parent) = path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::File::create(path)?;
                    }
                    self.undo_stack.push(action);
                }
                UndoAction::Move { .. } | UndoAction::Delete { .. } => {
                    // These shouldn't be in the stack, but if they are, ignore them
                }
            }
            self.load_entries()?;
        }
        Ok(())
    }

    fn read_input_with_escape(prompt: &str) -> io::Result<Option<String>> {
        use crossterm::event::{self, Event, KeyCode};

        execute!(io::stdout(), cursor::Show)?;

        print!("{}", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Esc => {
                            println!();
                            execute!(io::stdout(), cursor::Hide)?;
                            return Ok(None);
                        }
                        KeyCode::Enter => {
                            println!();
                            execute!(io::stdout(), cursor::Hide)?;
                            return Ok(Some(input));
                        }
                        KeyCode::Char(c) => {
                            input.push(c);
                            print!("{}", c);
                            io::stdout().flush()?;
                        }
                        KeyCode::Backspace => {
                            if !input.is_empty() {
                                input.pop();
                                print!("\x08 \x08");
                                io::stdout().flush()?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn create_new(&mut self) -> io::Result<()> {
        if let Some(input) = Self::read_input_with_escape("\nCreate (end with / for directory): ")? {
            let input = input.trim();

            if !input.is_empty() {
                let path = self.current_dir.join(input);
                let is_dir = input.ends_with('/');

                if is_dir {
                    // Create directory
                    fs::create_dir_all(&path)?;
                } else {
                    // Create file (touch)
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::File::create(&path)?;
                }

                self.undo_stack.push(UndoAction::Create {
                    path: path.clone(),
                    was_dir: is_dir
                });
                self.redo_stack.clear();
                self.load_entries()?;
            }
        }

        Ok(())
    }
}

fn show_welcome_pages() -> io::Result<()> {
    use crossterm::event::{self, Event, KeyCode};

    // Page 1: Welcome & General Info
    println!("\n{}", "=".repeat(60));
    println!("  Welcome to ils - Interactive ls");
    println!("{}", "=".repeat(60));
    println!("\nA fast, keyboard-driven file browser for the terminal.\n");
    println!("Features:");
    println!("  • Fuzzy find navigation");
    println!("  • File preview with syntax highlighting");
    println!("  • Image and PDF preview");
    println!("  • Permission editing, file operations, and more");
    println!("\nPress any key to continue...");

    terminal::enable_raw_mode()?;
    loop {
        if let Event::Key(_) = event::read()? {
            break;
        }
    }
    terminal::disable_raw_mode()?;

    // Page 2: Shell Integration Setup
    execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    println!("\n{}", "=".repeat(60));
    println!("  Shell Integration Setup");
    println!("{}", "=".repeat(60));
    println!("\nTo navigate directories, ils needs a shell wrapper function.");
    println!("This allows 'cd' to work when you exit the browser.\n");

    // Detect shell
    let shell = env::var("SHELL").unwrap_or_default();
    let rc_file = if shell.contains("zsh") {
        Some(format!("{}/.zshrc", env::var("HOME").unwrap_or_default()))
    } else if shell.contains("bash") {
        Some(format!("{}/.bashrc", env::var("HOME").unwrap_or_default()))
    } else {
        None
    };

    if let Some(ref rc_path) = rc_file {
        println!("Add shell integration to {}? (y/N): ", rc_path);
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if response.trim().to_lowercase() == "y" {
            let shell_function = r#"
# ils - Interactive ls
ils() {
    ils-bin "$@"
    if [ -f /tmp/ils_cd ]; then
        local target=$(cat /tmp/ils_cd)
        rm /tmp/ils_cd
        if [ -d "$target" ]; then
            cd "$target"
        else
            echo "$target"
        fi
    fi
}
"#;
            use std::fs::OpenOptions;
            use std::io::Write;

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&rc_path)?;

            file.write_all(shell_function.as_bytes())?;
            println!("\n✓ Shell integration added to {}", rc_path);
            println!("Run 'source {}' or restart your terminal.\n", rc_path);
        } else {
            println!("\nSkipped. Run 'ils --init' later to see the shell function.\n");
        }
    } else {
        println!("Could not detect shell. Run 'ils --init' to see the shell function.\n");
    }

    println!("Press any key to continue...");
    terminal::enable_raw_mode()?;
    loop {
        if let Event::Key(_) = event::read()? {
            break;
        }
    }
    terminal::disable_raw_mode()?;

    // Page 3: Quick Start / Help
    execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    println!("\n{}", "=".repeat(60));
    println!("  Quick Start Guide");
    println!("{}", "=".repeat(60));
    println!("\nBasic Navigation:");
    println!("  w/s/a/d or arrows  -  Navigate");
    println!("  Enter or l         -  Open file/directory");
    println!("  j or b             -  Go back");
    println!("  h                  -  Go home");
    println!("\nFuzzy Find:");
    println!("  /                  -  Jump mode (auto-exit)");
    println!("  Shift+/ or ?       -  Stay mode (keep searching)");
    println!("\nOther:");
    println!("  p                  -  Toggle preview");
    println!("  m                  -  Toggle list mode");
    println!("  Space              -  Toggle file info / line numbers");
    println!("  !                  -  Help menu");
    println!("  Esc                -  Quit without cd");
    println!("  q                  -  Exit and cd to directory");
    println!("\nPress any key to start...");

    terminal::enable_raw_mode()?;
    loop {
        if let Event::Key(_) = event::read()? {
            break;
        }
    }
    terminal::disable_raw_mode()?;

    // Create default config
    Config::create_default()?;
    execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    // Check for --version flag
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-v") {
        println!("ils v0.1.0");
        return Ok(());
    }

    // Check for --install flag
    if args.len() > 1 && args[1] == "--install" {
        return install();
    }

    // Check for config command
    if args.len() > 1 && args[1] == "config" {
        // Create default config if it doesn't exist
        if let Some(config_path) = Config::path() {
            if !config_path.exists() {
                Config::create_default()?;
            }

            // Open config in editor
            let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
            let status = std::process::Command::new(editor)
                .arg(&config_path)
                .status()?;

            if status.success() {
                println!("Config saved to: {}", config_path.display());
            }
        }
        return Ok(());
    }

    // Check for --init flag (legacy)
    if args.len() > 1 && args[1] == "--init" {
        println!(r#"# Interactive ls (ils) - Add this to your ~/.zshrc or ~/.bashrc
# NOTE: Replace 'ils-bin' with the actual path to your compiled binary if it's not in your PATH
ils() {{
    ils-bin "$@"
    if [ -f /tmp/ils_cd ]; then
        local target=$(cat /tmp/ils_cd)
        rm /tmp/ils_cd
        if [ -d "$target" ]; then
            cd "$target"
        else
            echo "$target"
        fi
    fi
}}
"#);
        return Ok(());
    }

    // Check for first run and show welcome pages
    let first_run = Config::path().map(|p| !p.exists()).unwrap_or(true);
    if first_run {
        show_welcome_pages()?;
    }

    let start_dir = env::current_dir()?;
    let mut browser = FileBrowser::new(start_dir)?;

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    // Use execute! for initial setup commands that should happen before the loop starts
    execute!(stdout, cursor::Hide)?;

    // We store the result as an Option<PathBuf> now
    let result = run_browser(&mut browser);

    // Clean up
    execute!(stdout, cursor::Show)?;
    terminal::disable_raw_mode()?;

    match result {
        Ok(ExitAction::Cd(final_path)) => {
            // Write to temp file for the shell wrapper to read.
            let _ = fs::write("/tmp/ils_cd", final_path.display().to_string());
        }
        Ok(ExitAction::OpenInFinder(final_path)) => {
            // Open directory in Finder
            let _ = std::process::Command::new("open")
                .arg(&final_path)
                .spawn();
        }
        Ok(ExitAction::None) => {
            // Quit without action (q)
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    // Small delay to ensure file write completes before shell reads it
    thread::sleep(Duration::from_millis(10));

    // Clear screen after delay
    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    Ok(())
}

enum ExitAction {
    None,
    Cd(PathBuf),
    OpenInFinder(PathBuf),
}

fn run_browser(browser: &mut FileBrowser) -> io::Result<ExitAction> {
    loop {
        browser.draw()?;

        match event::read()? {
            Event::Key(KeyEvent { code, modifiers, .. }) => {
                // Clear error message on any key press
                browser.error_message = None;
                // If help is showing, any key dismisses it
                if browser.show_help {
                    browser.show_help = false;
                    continue;
                }

                // Handle fuzzy find mode
                if browser.fuzzy_mode {
                    match code {
                        KeyCode::Esc => {
                            // Esc: exit fuzzy mode (don't exit the app)
                            browser.fuzzy_mode = false;
                            browser.fuzzy_query.clear();
                            browser.fuzzy_prev_count = 0;
                            continue;
                        }
                        KeyCode::Char('q') => {
                            // q: cd to current directory and exit
                            browser.fuzzy_mode = false;
                            browser.fuzzy_query.clear();
                            browser.fuzzy_prev_count = 0;
                            return Ok(ExitAction::Cd(browser.get_current_dir().clone()));
                        }
                        KeyCode::Char(ch) if browser.keybindings.contains(&browser.keybindings.quit_then_open_in_finder, ch) => {
                            // Open current directory in Finder and exit
                            browser.fuzzy_mode = false;
                            browser.fuzzy_query.clear();
                            browser.fuzzy_prev_count = 0;
                            return Ok(ExitAction::OpenInFinder(browser.get_current_dir().clone()));
                        }
                        KeyCode::Char(ch) if browser.keybindings.contains(&browser.keybindings.fuzzy_back, ch) => {
                            // Go back up a directory but stay in fuzzy mode
                            browser.fuzzy_query.clear();
                            browser.go_back()?;
                            browser.fuzzy_prev_count = browser.entries.len();
                            continue;
                        }
                        KeyCode::Char(ch) if browser.keybindings.contains(&browser.keybindings.fuzzy_home, ch) => {
                            // Go home but stay in fuzzy mode
                            browser.fuzzy_query.clear();
                            browser.go_home()?;
                            browser.fuzzy_prev_count = browser.entries.len();
                            continue;
                        }
                        KeyCode::Backspace => {
                            browser.fuzzy_query.pop();
                            let (match_idx, count) = browser.fuzzy_match();
                            // Only update selection if there's exactly one match
                            if count == 1 {
                                if let Some(idx) = match_idx {
                                    browser.selected = idx;
                                }
                            }
                            browser.fuzzy_prev_count = count;
                            continue;
                        }
                        KeyCode::Enter => {
                            // Enter: Same behavior as normal mode - open file in editor or cd to directory
                            browser.fuzzy_query.clear();
                            browser.fuzzy_prev_count = 0;
                            browser.fuzzy_mode = false;

                            if let Some(selected_path) = browser.get_selected_path() {
                                if selected_path.is_file() {
                                    // Write current directory to temp file for shell wrapper
                                    let _ = fs::write("/tmp/ils_cd", browser.get_current_dir().display().to_string());

                                    // Disable raw mode and open in default editor
                                    terminal::disable_raw_mode()?;
                                    execute!(io::stdout(), cursor::Show)?;

                                    let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                                    let _ = std::process::Command::new(editor)
                                        .arg(&selected_path)
                                        .status();

                                    // Check if we should exit after editing
                                    if browser.settings.exit_after_edit {
                                        return Ok(ExitAction::Cd(browser.get_current_dir().clone()));
                                    }

                                    // Re-enable raw mode
                                    execute!(io::stdout(), cursor::Hide)?;
                                    terminal::enable_raw_mode()?;
                                } else {
                                    // It's a directory, exit with it
                                    return Ok(ExitAction::Cd(selected_path));
                                }
                            } else {
                                // No selection, return current directory
                                return Ok(ExitAction::Cd(browser.get_current_dir().clone()));
                            }
                            continue;
                        }
                        KeyCode::Up => {
                            browser.select_up();
                            continue;
                        }
                        KeyCode::Down => {
                            browser.select_down();
                            continue;
                        }
                        KeyCode::Left => {
                            browser.select_left();
                            continue;
                        }
                        KeyCode::Right => {
                            browser.select_right();
                            continue;
                        }
                        KeyCode::Char(ch) => {
                            browser.fuzzy_query.push(ch);
                            let (match_idx, count) = browser.fuzzy_match();
                            // Only update selection if there's exactly one match
                            if count == 1 {
                                if let Some(idx) = match_idx {
                                    browser.selected = idx;
                                }
                                // Auto-open if we narrowed down to 1 match
                                if browser.fuzzy_prev_count > 1 || browser.fuzzy_prev_count == 1 {
                                    browser.fuzzy_query.clear();
                                    browser.open_selected()?;

                                    if browser.fuzzy_jump_mode {
                                        // Exit fuzzy mode after jump
                                        browser.fuzzy_mode = false;
                                        browser.fuzzy_prev_count = 0;
                                    } else {
                                        // Stay in fuzzy mode and reset count to new directory's entry count
                                        browser.fuzzy_prev_count = browser.entries.len();
                                    }

                                    // Drain any pending keyboard events to prevent accidental typing
                                    let drain_start = std::time::Instant::now();
                                    while drain_start.elapsed() < Duration::from_millis(500) {
                                        if event::poll(Duration::from_millis(50))? {
                                            if let Event::Key(_) = event::read()? {
                                                // Discard buffered key events
                                            }
                                        }
                                    }
                                } else {
                                    browser.fuzzy_prev_count = count;
                                }
                            } else {
                                browser.fuzzy_prev_count = count;
                            }
                            continue;
                        }
                        _ => continue,
                    }
                }

                // Check character-based bindings first
                if let KeyCode::Char(ch) = code {
                    if browser.keybindings.contains(&browser.keybindings.help, ch) || ch == '!' {
                        browser.show_help = !browser.show_help;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.quit, ch) {
                        return Ok(ExitAction::Cd(browser.get_current_dir().clone()));
                    }
                    if browser.keybindings.contains(&browser.keybindings.quit_then_open_in_finder, ch) {
                        // Open current directory in Finder and exit
                        return Ok(ExitAction::OpenInFinder(browser.get_current_dir().clone()));
                    }
                    if browser.keybindings.contains(&browser.keybindings.up, ch) {
                        browser.select_up();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.down, ch) {
                        browser.select_down();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.left, ch) {
                        browser.select_left();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.right, ch) {
                        browser.select_right();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.jump_up, ch) {
                        browser.jump_up();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.jump_down, ch) {
                        browser.jump_down();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.jump_left, ch) {
                        browser.jump_left();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.jump_right, ch) {
                        browser.jump_right();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.open, ch) {
                        browser.open_selected()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.back, ch) {
                        browser.go_back()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.home, ch) {
                        browser.go_home()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.next_sibling, ch) {
                        browser.go_to_next_sibling()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.prev_sibling, ch) {
                        browser.go_to_prev_sibling()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.preview_toggle, ch) {
                        browser.preview_mode = !browser.preview_mode;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.toggle_hidden, ch) {
                        browser.show_hidden = !browser.show_hidden;
                        browser.load_entries()?;
                        browser.update_layout()?;
                        let _ = browser.save_show_hidden();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.toggle_mode, ch) {
                        browser.list_mode = !browser.list_mode;
                        browser.update_layout()?;
                        continue;
                    }
                    if ch == 'e' && browser.list_mode {
                        if browser.list_info_mode == 1 {
                            // Toggle between modified and created date when in date mode
                            browser.show_created_date = !browser.show_created_date;
                        } else if browser.list_info_mode == 2 {
                            // Edit permissions when in permissions mode
                            if let Some(selected_path) = browser.get_selected_path() {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    if let Ok(metadata) = selected_path.metadata() {
                                        let current_mode = metadata.permissions().mode() & 0o777;

                                        if let Ok(Some(input)) = FileBrowser::read_input_with_escape(&format!("\nCurrent permissions: {:o}\nEnter new permissions (octal, e.g., 755): ", current_mode)) {
                                            if let Ok(new_mode) = u32::from_str_radix(input.trim(), 8) {
                                                if new_mode <= 0o777 {
                                                    use std::fs::Permissions;
                                                    if let Err(e) = fs::set_permissions(&selected_path, Permissions::from_mode(new_mode)) {
                                                        terminal::disable_raw_mode()?;
                                                        eprintln!("Error setting permissions: {}", e);
                                                        std::thread::sleep(std::time::Duration::from_secs(2));
                                                        terminal::enable_raw_mode()?;
                                                    }
                                                }  else {
                                                    terminal::disable_raw_mode()?;
                                                    eprintln!("Invalid permissions value");
                                                    std::thread::sleep(std::time::Duration::from_secs(2));
                                                    terminal::enable_raw_mode()?;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else if browser.list_info_mode == 3 {
                            // Calculate directory sizes when in size mode
                            browser.calculate_all_dir_sizes()?;
                        }
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.copy, ch) {
                        browser.copy_to_clipboard();
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.paste, ch) {
                        browser.paste_from_clipboard()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.trash, ch) {
                        browser.move_to_trash()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.delete, ch) {
                        browser.delete_permanent()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.undo, ch) {
                        browser.undo()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.redo, ch) {
                        browser.redo()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.create, ch) {
                        browser.create_new()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.rename, ch) {
                        // Rename functionality
                        if let Some(selected_path) = browser.get_selected_path() {
                            if let Some(old_name) = selected_path.file_name().and_then(|n| n.to_str()) {
                                if let Ok(Some(new_name)) = FileBrowser::read_input_with_escape(&format!("\nRename '{}' to: ", old_name)) {
                                    let new_name = new_name.trim();

                                    if !new_name.is_empty() && new_name != old_name {
                                        let new_path = selected_path.parent().unwrap().join(new_name);
                                        if let Err(e) = fs::rename(&selected_path, &new_path) {
                                            terminal::disable_raw_mode()?;
                                            eprintln!("Error renaming: {}", e);
                                            std::thread::sleep(std::time::Duration::from_secs(2));
                                            terminal::enable_raw_mode()?;
                                        } else {
                                            browser.undo_stack.push(UndoAction::Rename {
                                                old_path: selected_path.clone(),
                                                new_path: new_path.clone()
                                            });
                                            browser.redo_stack.clear();
                                            browser.load_entries()?;
                                        }
                                    }
                                }
                            }
                        }
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.fuzzy_find, ch) || browser.keybindings.contains(&browser.keybindings.fuzzy_home, ch) {
                        browser.fuzzy_mode = true;
                        browser.fuzzy_query.clear();
                        browser.fuzzy_prev_count = browser.entries.len();
                        // fuzzy_home (?) always uses stay mode, fuzzy_find (/) uses jump mode unless Shift is held
                        browser.fuzzy_jump_mode = browser.keybindings.contains(&browser.keybindings.fuzzy_find, ch) && !modifiers.contains(KeyModifiers::SHIFT);
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.preview_height_decrease, ch) {
                        if browser.preview_mode {
                            browser.preview_split_ratio = (browser.preview_split_ratio - 0.1).max(0.2);
                            let _ = browser.save_preview_ratio();
                        }
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.preview_height_increase, ch) {
                        if browser.preview_mode {
                            browser.preview_split_ratio = (browser.preview_split_ratio + 0.1).min(1.0);
                            let _ = browser.save_preview_ratio();
                        }
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.preview_up, ch) || ch == 'I' {
                        // Scroll preview up - shift for visible lines (uppercase), otherwise configured amount
                        if browser.preview_mode {
                            if let Some(selected) = browser.get_selected_path() {
                                let (_, height) = terminal::size()?;
                                let split_line = browser.start_row + ((height - browser.start_row) as f32 * (1.0 - browser.preview_split_ratio)) as u16;
                                let preview_lines = (height - split_line - 3) as usize;

                                let scroll_amount = if ch == 'I' || modifiers.contains(KeyModifiers::SHIFT) {
                                    preview_lines
                                } else {
                                    browser.settings.preview_scroll_amount
                                };

                                let current = browser.preview_scroll_map.get(&selected).copied().unwrap_or(0);
                                let new_scroll = current.saturating_sub(scroll_amount);
                                browser.preview_scroll_map.insert(selected, new_scroll);
                            }
                        }
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.preview_down, ch) || ch == 'O' {
                        // Scroll preview down - shift for visible lines (uppercase), otherwise configured amount
                        if browser.preview_mode {
                            if let Some(selected) = browser.get_selected_path() {
                                if selected.is_file() {
                                    let (_, height) = terminal::size()?;
                                    let split_line = browser.start_row + ((height - browser.start_row) as f32 * (1.0 - browser.preview_split_ratio)) as u16;
                                    let preview_lines = (height - split_line - 3) as usize;

                                    let scroll_amount = if ch == 'O' || modifiers.contains(KeyModifiers::SHIFT) {
                                        preview_lines
                                    } else {
                                        browser.settings.preview_scroll_amount
                                    };

                                    // Get file line count to bound scroll
                                    if let Ok(file) = fs::File::open(&selected) {
                                        use io::BufRead;
                                        let line_count = io::BufReader::new(file).lines().count();

                                        let current = browser.preview_scroll_map.get(&selected).copied().unwrap_or(0);
                                        // Don't scroll past the last visible line
                                        let max_scroll = line_count.saturating_sub(preview_lines);
                                        let new_scroll = (current + scroll_amount).min(max_scroll);
                                        browser.preview_scroll_map.insert(selected, new_scroll);
                                    }
                                }
                            }
                        }
                        continue;
                    }
                }

                // Handle arrow keys and special keys
                match code {
                    KeyCode::Esc => {
                        // Esc: quit without cd
                        return Ok(ExitAction::None);
                    }
                    KeyCode::Up => browser.select_up(),
                    KeyCode::Down => browser.select_down(),
                    KeyCode::Left => browser.select_left(),
                    KeyCode::Right => browser.select_right(),
                    KeyCode::Char(' ') => {
                        // Space: Toggle list info mode in list mode, or line numbers in preview mode
                        if browser.preview_mode {
                            browser.show_line_numbers = !browser.show_line_numbers;
                        } else {
                            browser.list_info_mode = (browser.list_info_mode + 1) % 4;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char('k') => {
                        // Enter: Select item - if file, open in editor; if directory, cd to it
                        if let Some(selected_path) = browser.get_selected_path() {
                            if selected_path.is_file() {
                                // Write current directory to temp file for shell wrapper
                                let _ = fs::write("/tmp/ils_cd", browser.get_current_dir().display().to_string());

                                // Disable raw mode and open in default editor
                                terminal::disable_raw_mode()?;
                                execute!(io::stdout(), cursor::Show)?;

                                let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                                let _ = std::process::Command::new(editor)
                                    .arg(&selected_path)
                                    .status();

                                // Check if we should exit after editing
                                if browser.settings.exit_after_edit {
                                    return Ok(ExitAction::Cd(browser.get_current_dir().clone()));
                                }

                                // Re-enable raw mode
                                execute!(io::stdout(), cursor::Hide)?;
                                terminal::enable_raw_mode()?;
                            } else {
                                // It's a directory, exit with it
                                return Ok(ExitAction::Cd(selected_path));
                            }
                        } else {
                            // No selection, return current directory
                            return Ok(ExitAction::Cd(browser.get_current_dir().clone()));
                        }
                    }
                    KeyCode::Backspace => { browser.go_back()?; }
                    _ => {}
                }
            }
            Event::Resize(_, _) => {
                browser.update_layout()?; // Recalculate columns on resize
            }
            _ => {}
        }
    }
}
