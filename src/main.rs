use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
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
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::as_24_bit_terminal_escaped,
};
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
    help: Vec<char>,
    preview_toggle: Vec<char>,
    preview_up: Vec<char>,
    preview_down: Vec<char>,
    preview_height_decrease: Vec<char>,
    preview_height_increase: Vec<char>,
    toggle_hidden: Vec<char>,
    fuzzy_find: Vec<char>,
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
    #[serde(default = "default_preview_border_fg")]
    preview_border_fg: String,
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

fn default_preview_border_fg() -> String {
    "darkgrey".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
struct Settings {
    #[serde(default = "default_exit_after_edit")]
    exit_after_edit: bool,
    #[serde(default = "default_preview_scroll_amount")]
    preview_scroll_amount: usize,
}

fn default_exit_after_edit() -> bool {
    false
}

fn default_preview_scroll_amount() -> usize {
    10
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            exit_after_edit: default_exit_after_edit(),
            preview_scroll_amount: default_preview_scroll_amount(),
        }
    }
}

impl Settings {
    fn load() -> Self {
        if let Ok(home) = env::var("HOME") {
            let config_path = PathBuf::from(home).join(".config/ils/settings.toml");
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
            preview_border_fg: default_preview_border_fg(),
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

    fn parse_preview_border_fg(&self) -> Option<Color> {
        Self::parse_color_string(&self.preview_border_fg)
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
            help: vec!['?'],
            preview_toggle: vec!['p'],
            preview_up: vec!['i'],
            preview_down: vec!['o'],
            preview_height_decrease: vec!['-', '_'],
            preview_height_increase: vec!['+', '='],
            toggle_hidden: vec!['.'],
            fuzzy_find: vec!['/'],
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
    keybindings: Keybindings,
    color_config: ColorConfig,
    settings: Settings,
    preview_cache: HashMap<PathBuf, Vec<String>>, // Cache preview content
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl FileBrowser {
    fn new(start_dir: PathBuf) -> io::Result<Self> {
        let (_, row) = cursor::position()?;

        // Load saved preview split ratio
        let preview_split_ratio = Self::load_preview_ratio().unwrap_or(0.5);

        // Check if this is first run (show help if no config exists)
        let show_help = !Self::config_exists();

        // Load keybindings or create defaults if not exists
        let keybindings = if Self::config_exists() {
            Keybindings::load()
        } else {
            let defaults = Keybindings::default();
            let _ = defaults.save();
            defaults
        };

        // Load color config or create defaults if not exists
        let color_config = if Self::color_config_exists() {
            ColorConfig::load()
        } else {
            let defaults = ColorConfig::default();
            let _ = defaults.save();
            defaults
        };

        // Load settings
        let settings = Settings::load();

        // start drawing content on the row *after* the initial position
        let mut browser = FileBrowser {
            current_dir: start_dir,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            num_cols: 1,
            start_row: row,
            breadcrumbs: Vec::new(),
            show_dir_slash: false, // Config: show trailing slash for directories
            preview_mode: false,
            preview_scroll_map: HashMap::new(),
            preview_split_ratio,
            show_help,
            show_hidden: false,
            fuzzy_mode: false,
            fuzzy_query: String::new(),
            fuzzy_prev_count: 0,
            keybindings,
            color_config,
            settings,
            preview_cache: HashMap::new(),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        };
        browser.load_entries()?;
        browser.update_layout()?; // Initial layout calculation

        // Mark that config exists now
        if show_help {
            let _ = browser.save_preview_ratio();
        }

        Ok(browser)
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

        // Calculate max columns that can fit
        let max_cols = (term_width / CELL_WIDTH).max(1);

        // Calculate available rows for content (subtract header and footer)
        let available_rows = term_height.saturating_sub(self.start_row as usize).saturating_sub(2);

        // Layout method: Square
        // Distribute into a square-like layout (cols ≈ rows)
        let num_entries = self.entries.len().max(1);

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

        // Ensure selected index is valid after layout change
        if !self.entries.is_empty() {
            self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        }

        Ok(())
    }

    fn draw(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();

        let (width, height) = terminal::size()?;

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

        if fg_color.is_none() && bg_color.is_none() {
            // Use reverse attribute (default)
            queue!(
                stdout,
                crossterm::style::SetAttribute(crossterm::style::Attribute::Reverse),
                Print(format!(" {} ", self.current_dir.display())),
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
            queue!(stdout, Print(format!(" {} ", self.current_dir.display())))?;
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
            let num_rows = ((self.entries.len() + self.num_cols - 1) / self.num_cols).min(max_display_rows);

            for row in 0..num_rows {
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

                    if display_name.len() > NAME_WIDTH {
                        display_name.truncate(NAME_WIDTH - 1);
                        display_name.push('~');
                    }

                    let prefix = if is_selected { "> " } else { "  " };

                    // Check if this entry matches the fuzzy query
                    let query_len = if self.fuzzy_mode && !self.fuzzy_query.is_empty() {
                        let query_lower = self.fuzzy_query.to_lowercase();
                        if name.to_lowercase().starts_with(&query_lower) {
                            self.fuzzy_query.len().min(display_name.len())
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    // Print prefix
                    queue!(stdout, Print(prefix))?;

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
                        // Apply directory color
                        if let Some(fg) = self.color_config.parse_directory_fg() {
                            queue!(stdout, SetForegroundColor(fg))?;
                        } else {
                            queue!(stdout, SetForegroundColor(Color::Blue))?;
                        }
                    } else {
                        queue!(stdout, ResetColor)?;
                    }

                    // Print name with fuzzy match highlighting
                    if query_len > 0 {
                        // Print matching part in bright yellow with bold and dark background
                        queue!(stdout, crossterm::style::SetAttribute(crossterm::style::Attribute::Bold))?;
                        queue!(stdout, SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 0 }))?;
                        queue!(stdout, crossterm::style::SetBackgroundColor(Color::Rgb { r: 50, g: 50, b: 50 }))?;
                        queue!(stdout, Print(&display_name[..query_len]))?;
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
                        let rest = &display_name[query_len..];
                        let padding = NAME_WIDTH - display_name.len();
                        queue!(stdout, Print(format!("{}{}", rest, " ".repeat(padding))))?;
                    } else {
                        // No match, print normally with padding
                        queue!(stdout, Print(format!("{:<width$}", display_name, width = NAME_WIDTH)))?;
                    }

                    queue!(stdout, ResetColor)?;
                }
                queue!(stdout, Print("\r\n"))?;
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
                if selected.is_file() {
                    let preview_lines = (height - split_line - 3) as usize;

                    // Read only what we need from the file
                    if let Ok(file) = fs::File::open(&selected) {
                        use io::BufRead;
                        let reader = io::BufReader::new(file);
                        let scroll_pos = self.preview_scroll_map.get(&selected).copied().unwrap_or(0);

                        // Try to detect syntax
                        let syntax = self.syntax_set
                            .find_syntax_for_file(&selected)
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

                        let theme = &self.theme_set.themes["base16-ocean.dark"];
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

                            // Highlight the line
                            let ranges = highlighter.highlight_line(line, &self.syntax_set).unwrap_or_default();
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
            "  l / Enter          -  Open directory",
            "  j / b / Backspace  -  Go back",
            "  h                  -  Go home",
            "  Space              -  Select (open file in $EDITOR or cd to dir)",
            "",
            "PREVIEW & COMMANDS:",
            "  p                  -  Toggle preview",
            "  i / o              -  Scroll preview up/down (10 lines)",
            "  Shift+I / Shift+O  -  Scroll preview faster",
            "  - / +              -  Decrease/increase preview height",
            "  .                  -  Toggle hidden files",
            "  ?                  -  Toggle this help",
            "  q / Esc            -  Quit",
            "",
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
        }
    }

    fn select_down(&mut self) {
        // Row-major: move down one row (add num_cols)
        let new_idx = self.selected + self.num_cols;
        if new_idx < self.entries.len() {
            self.selected = new_idx;
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

    fn fuzzy_match(&self) -> (Option<usize>, usize) {
        // Find all entries that match the fuzzy query and return (first_match, count)
        if self.fuzzy_query.is_empty() {
            return (None, 0);
        }

        let query_lower = self.fuzzy_query.to_lowercase();

        let matches: Vec<usize> = self.entries.iter().enumerate()
            .filter_map(|(idx, entry)| {
                if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
                    if name.to_lowercase().starts_with(&query_lower) {
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
            // Add the selected folder to breadcrumbs before navigating
            if let Some(folder_name) = selected_path.file_name().and_then(|n| n.to_str()) {
                self.breadcrumbs.push(folder_name.to_string());
            }
            self.current_dir = selected_path.clone();
            self.load_entries()?;
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
                            // q: quit without cd
                            browser.fuzzy_mode = false;
                            browser.fuzzy_query.clear();
                            browser.fuzzy_prev_count = 0;
                            return Ok(ExitAction::None);
                        }
                        KeyCode::Char('Q') => {
                            // Shift+Q: open current directory in Finder and exit
                            browser.fuzzy_mode = false;
                            browser.fuzzy_query.clear();
                            browser.fuzzy_prev_count = 0;
                            return Ok(ExitAction::OpenInFinder(browser.get_current_dir().clone()));
                        }
                        KeyCode::Char('/') => {
                            // Go back up a directory but stay in fuzzy mode
                            browser.fuzzy_query.clear();
                            browser.go_back()?;
                            browser.fuzzy_prev_count = browser.entries.len();
                            continue;
                        }
                        KeyCode::Char('?') => {
                            // Shift+/ (which produces '?') - go home but stay in fuzzy mode
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
                                    // Stay in fuzzy mode and reset count to new directory's entry count
                                    browser.fuzzy_prev_count = browser.entries.len();
                                    // Drain any pending keyboard events to prevent accidental typing
                                    thread::sleep(Duration::from_millis(200));
                                    while event::poll(Duration::from_millis(0))? {
                                        if let Event::Key(_) = event::read()? {
                                            // Discard buffered key events
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
                    if browser.keybindings.contains(&browser.keybindings.help, ch) {
                        browser.show_help = !browser.show_help;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.quit, ch) {
                        return Ok(ExitAction::None);
                    }
                    if ch == 'Q' {
                        // Shift+Q: open current directory in Finder and exit
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
                    if browser.keybindings.contains(&browser.keybindings.preview_toggle, ch) {
                        browser.preview_mode = !browser.preview_mode;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.toggle_hidden, ch) {
                        browser.show_hidden = !browser.show_hidden;
                        browser.load_entries()?;
                        browser.update_layout()?;
                        continue;
                    }
                    if browser.keybindings.contains(&browser.keybindings.fuzzy_find, ch) {
                        browser.fuzzy_mode = true;
                        browser.fuzzy_query.clear();
                        browser.fuzzy_prev_count = browser.entries.len();
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
                        // Esc: cd to current directory and exit
                        return Ok(ExitAction::Cd(browser.get_current_dir().clone()));
                    }
                    KeyCode::Up => browser.select_up(),
                    KeyCode::Down => browser.select_down(),
                    KeyCode::Left => browser.select_left(),
                    KeyCode::Right => browser.select_right(),
                    KeyCode::Enter => {
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
