//! A command-line utility that builds the binaries and copies them in the book folder.
use std::{fs::{copy, create_dir, remove_dir_all}, path::{Path, PathBuf}};
use serde::Deserialize;

#[derive(Deserialize)]
struct Message {
    reason: String,
    executable: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Manifest {
    manifest_path: PathBuf,
}

mod run_command;
use run_command::{get_command_output, run_command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_json = get_command_output(&["cargo", "read-manifest"])?;
    let manifest: Manifest = serde_json::from_str(manifest_json.as_str())?;
    let base_folder_str = manifest.manifest_path.parent().ok_or("could not get the project folder")?.as_os_str().to_str().unwrap();

    let mut targets = vec![
        ("windows", "x86_64-pc-windows-gnu"),
        ("mac", "aarch64-apple-darwin"),
        ("linux", "x86_64-unknown-linux-gnu"),
    ];
    let mut current = false;
    #[allow(unused_assignments)]
    let mut current_toolchain = String::new();
    if std::env::args().any(| x | x == "--current") {
        current = true;
        current_toolchain = get_command_output(&["rustup", "show", "active-toolchain"])?;
        current_toolchain = current_toolchain
            .split_once('-')
            .unwrap_or(("", &current_toolchain))
            .1.to_string();
        current_toolchain = current_toolchain
            .split_once(' ')
            .unwrap_or((&current_toolchain, ""))
            .0.to_string();
        let mut found_target = false;
        for (platform_name, target) in &targets {
            if current_toolchain.contains(target) {
                found_target = true;
                targets = vec![(platform_name, &current_toolchain)];
                break;
            }
        }
        if !found_target {
            targets = vec![("", &current_toolchain)];
        }
    }

    eprintln!("Installing toolchains...");
    let mut command = vec!["rustup", "target", "add"];
    for (_, target) in &targets {
        command.push(target);
    }
    run_command(&command)?;

    eprintln!("Building...");
    let mut command = vec!["cargo", "build", "--release", "--message-format", "json"];
    if std::env::args().any(| x | x == "--verbose") {
        command.push("--verbose");
    }
    for (_, target) in &targets {
        command.push("--target");
        command.push(target);
    }
    let stdout = get_command_output(&command)?;

    let folder = PathBuf::from("docs/src/dist");
    if folder.exists() {
        eprintln!("Removing {:?}...", folder.as_os_str());
        remove_dir_all(folder.clone())?;
    }
    create_dir(folder.clone())?;

    for line in stdout.split('\n') {
        if line.is_empty() {
            continue;
        }
        let data: Message = serde_json::from_str(line)?;
        if data.reason != "compiler-artifact" {
            continue;
        }
        if let Some(executable) = data.executable {
            let mut suffix = "";
            for (platform_name, platform) in &targets {
                let text_to_match = executable.to_str().unwrap().replace(base_folder_str, "");
                if text_to_match.contains(platform) {
                    suffix = platform_name;
                    break;
                }
            }
            if suffix.is_empty() && !current {
                eprintln!("WARNING: could not find suffix to add to executable");
            }
            eprintln!("Copying {:?} to {:?}...", executable, folder.as_os_str());
            let mut dest = Path::join(&folder, executable.file_name().unwrap());
            let file_name = format!(
                "{}{}{}",
                dest.file_stem().unwrap_or_default().to_string_lossy(),
                if suffix.is_empty() {String::new()} else {"-".to_owned() + suffix},
                dest.extension().map(| x | ".".to_owned() + x.to_str().unwrap_or("")).unwrap_or_default(),
            );
            dest.set_file_name(&file_name);
            copy(&executable, &dest)?;
        }
    }

    Ok(())
}
