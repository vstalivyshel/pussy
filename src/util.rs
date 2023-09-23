use anyhow::Context;
use chrono::offset::Local;
use crossterm::{
    cursor::MoveTo,
    terminal::{Clear, ClearType},
};
use std::{
    fs::File,
    io::Write,
};

pub fn clear_screen() {
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, MoveTo(0, 0), Clear(ClearType::All));
    let _ = stdout.flush();
}

pub fn create_file_cwd(name: &str) -> anyhow::Result<File> {
    let mut target = std::env::current_dir().context("Failed to get current dir")?;
    let file_name = if name.starts_with('.') {
        format!("{file_name}{ext}", file_name = Local::now().time(), ext = name)
    } else {
        name.to_string()
    };
    target.push(file_name);

    File::create(target).context("Failed to create file")
}
