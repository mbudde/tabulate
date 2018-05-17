use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut path = PathBuf::new();
    path.push(env::var("OUT_DIR").unwrap());
    path.push("build-info.txt");

    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .expect("Failed to execute command");

    let hash = &output.stdout[0..8];

    let output = Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .expect("Failed to execute command");

    let date = &output.stdout[0..10];

    let mut file = File::create(path).unwrap();
    file.write_all(hash).unwrap();
    file.write_all(b" ").unwrap();
    file.write_all(date).unwrap();
}
