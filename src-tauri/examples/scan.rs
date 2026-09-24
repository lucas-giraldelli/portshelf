//! Lists which catalog ports the game files in a folder match:
//! cargo run --example scan -- <folder>
#[path = "../src/romscan.rs"]
mod romscan;

fn main() {
    let dir = std::env::args().nth(1).expect("folder");
    let catalog: serde_json::Value = serde_json::from_str(include_str!("../../catalog/ports.json")).unwrap();
    for f in romscan::scan(std::path::Path::new(&dir), &catalog) {
        println!("{:10} {:5} {:9} {:6} {:28} {}", f.port, f.by, if f.modified { "MODIFIED" } else { "" }, if f.wrong_version { "WRONG" } else { "" }, f.detail, f.path);
    }
}
