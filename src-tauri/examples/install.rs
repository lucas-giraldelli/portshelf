//! Installs one catalog port into a scratch folder, to test the installer without the UI:
//! cargo run --example install -- <port id> <dest dir>
use std::path::PathBuf;

#[path = "../src/install.rs"]
mod install;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (id, dest) = (&args[1], PathBuf::from(&args[2]));
    let catalog: serde_json::Value = serde_json::from_str(include_str!("../../catalog/ports.json")).unwrap();
    let port = catalog["ports"].as_array().unwrap().iter().find(|p| p["id"] == id.as_str()).expect("unknown id");
    let rule: install::Rule = serde_json::from_value(port["install"]["linux"].clone()).expect("no linux rule");
    let work = dest.with_extension("work");
    match install::install(port["repo"].as_str().unwrap(), &rule, &dest.join(id), &work, |p| {
        if let install::Progress::Downloading { done, total } = p {
            eprint!("\r{} / {:?} bytes", done, total);
        }
    }) {
        Ok((tag, exec)) => println!("\n{id}: {tag} -> {}", exec.display()),
        Err(e) => println!("\n{id}: ERROR {e}"),
    }
}
