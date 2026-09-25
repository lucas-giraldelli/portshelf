// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `portshelf --overlay <json>` shows one achievement card over the running game and exits;
    // `--overlay-png <json> <file> [scale]` renders the card to an image instead.
    match args.get(1).map(String::as_str) {
        Some("--overlay") => std::process::exit(if portshelf_lib::overlay::run(args.get(2).map_or("", String::as_str)) { 0 } else { 1 }),
        Some("--overlay-png") => {
            let scale = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            if let Err(e) = portshelf_lib::overlay::render_png(&args[2], &args[3], scale) {
                eprintln!("{e}");
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }
    let _ = args;
    portshelf_lib::run()
}
