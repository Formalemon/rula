# rula

A custom application launcher and file browser written in Rust.
> specific to ArchLinux Hyprland rice with Kitty :D

## Features
- **Apps Mode**: Fuzzy search and launch applications.
- **Files Mode**: Fast, async file search (fd-like performance).
- **TUI**: Custom rendering engine using `crossterm`.
- **Persistent State**: SQLite database tracks usage and preferences.

## Building
```bash
cargo build
```

## Usage
```bash
kitty -e <path-to-rula>rula
```
- Enter: Launch App / Open file in NVIM
- Tab: Cycle between App and File mode.
- Ctrl+t: Toggle App Launch mode for Terminal App.

> For Terminal apps it spawns a kitty instance to run it.
> It will remember the Launch Mode for each App if set (defaults to direct exection).
