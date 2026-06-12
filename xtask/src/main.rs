use std::path::{Path, PathBuf};
use std::{env, fs, process};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must live one level below workspace root")
        .to_path_buf()
}

fn read_cargo_doc() -> (PathBuf, toml_edit::DocumentMut) {
    let path = workspace_root().join("Cargo.toml");
    let src = fs::read_to_string(&path).expect("read Cargo.toml");
    let doc: toml_edit::DocumentMut = src.parse().expect("parse Cargo.toml");
    (path, doc)
}

fn workspace_version(doc: &toml_edit::DocumentMut) -> semver::Version {
    doc.get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("workspace.package.version missing")
        .parse()
        .expect("invalid workspace.package.version")
}

fn update_readme_version_badge(old: &semver::Version, new: &semver::Version) {
    let readme = workspace_root().join("README.md");
    let Ok(text) = fs::read_to_string(&readme) else {
        return;
    };
    let updated = text.replace(&format!("version-v{old}-"), &format!("version-v{new}-"));
    if updated != text {
        fs::write(&readme, updated).expect("write README.md");
    }
}

fn bump_version(kind: &str) {
    let (path, mut doc) = read_cargo_doc();
    let old = workspace_version(&doc);
    let new = match kind {
        "minor" => semver::Version::new(old.major, old.minor + 1, 0),
        "patch" => semver::Version::new(old.major, old.minor, old.patch + 1),
        other => {
            eprintln!("error: expected `minor` or `patch`, got `{other}`");
            process::exit(2);
        }
    };

    let version = doc["workspace"]["package"]
        .get_mut("version")
        .expect("workspace.package.version missing");
    *version = toml_edit::value(new.to_string());
    fs::write(&path, doc.to_string()).expect("write Cargo.toml");
    update_readme_version_badge(&old, &new);
    println!("{new}");
}

fn read_version() {
    let (_path, doc) = read_cargo_doc();
    println!("{}", workspace_version(&doc));
}

fn usage() -> ! {
    eprintln!(
        "usage: cargo xtask <task>\n\n\
         tasks:\n\
           bump-version minor|patch  Bump workspace semver\n\
           read-version              Print current workspace version"
    );
    process::exit(2)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("bump-version") if args.len() == 3 => bump_version(&args[2]),
        Some("bump-version") => usage(),
        Some("read-version") if args.len() == 2 => read_version(),
        Some("read-version") => usage(),
        Some(other) => {
            eprintln!("unknown task: {other}");
            usage();
        }
        None => usage(),
    }
}
