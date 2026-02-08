// ============================================================================
// Main Entry Point - Optimized for Fast Startup
// ============================================================================

mod app;
mod db;
mod input;
mod system;
mod terminal;
mod theme;
mod ui;

use std::env;
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use app::App;
use input::InputHandler;
use ui::Ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for seed flag
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--seed" {
        let db = db::Database::new()?;
        system::seed_database(&db);
        println!("Done! Now run the launcher normally.");
        return Ok(());
    }

    // Rebuild app cache flag
    if args.len() > 1 && args[1] == "--rebuild-cache" {
        let db = db::Database::new()?;
        system::rebuild_app_cache(&db)?;
        println!("Cache rebuilt successfully!");
        return Ok(());
    }

    enable_raw_mode()?;

    // Fast startup - only load cached apps, files are lazy-loaded
    let mut app = App::new();
    let mut ui = Ui::new()?;
    let input_handler = InputHandler::new();

    let mut should_render = true;

    loop {
        if should_render {
            ui.render(&app)?;
            should_render = false;
        }

        if app.should_quit {
            break;
        }

        if app.should_launch {
            if let Some((program, args, is_tui)) = app.launch_command.take() {
                disable_raw_mode()?;
                spawn_detached(&program, &args, is_tui);
                break;
            }
        }

        // Poll with long timeout to prevent busy-waiting
        if let Some(key) = input_handler.poll(100_000) {
            input_handler.process(&mut app, key);
            should_render = true;
        }
    }

    disable_raw_mode()?;
    Ok(())
}

fn spawn_detached(program: &str, args: &[String], is_tui: bool) {
    let final_program: String;
    let final_args: Vec<String>;

    if is_tui {
        final_program = "kitty".to_string();
        let mut kitty_args = vec!["-e".to_string(), program.to_string()];
        kitty_args.extend(args.iter().cloned());
        final_args = kitty_args;
    } else {
        final_program = program.to_string();
        final_args = args.to_vec();
    }

    let args_refs: Vec<&str> = final_args.iter().map(|s| s.as_str()).collect();

    unsafe {
        let _ = Command::new(&final_program)
            .args(&args_refs)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .pre_exec(|| {
                libc::setsid();
                Ok(())
            })
            .spawn();
    }
}
