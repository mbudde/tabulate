use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("build-info.txt");

    let hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|out| out.stdout)
        .map(|mut hash| {
            hash.truncate(9);
            hash
        });

    let date = Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .map(|out| out.stdout)
        .map(|mut date| {
            date.truncate(10);
            date
        });

    let mut file = File::create(path).unwrap();
    if let (Ok(h), Ok(d)) = (hash, date) {
        file.write_all(b" (").unwrap();
        file.write_all(&h).unwrap();
        file.write_all(b" ").unwrap();
        file.write_all(&d).unwrap();
        file.write_all(b")").unwrap();
    }
}
