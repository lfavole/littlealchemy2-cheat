//! A command-line utility that builds the binaries and the book.
use std::{ffi::OsStr, fs::{copy, create_dir, metadata, remove_dir_all, write}, path::{Path, PathBuf}, process::{exit, Command, Stdio}};
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

#[allow(clippy::cast_precision_loss)]
/// Format a file size to make it human-readable.
fn format_file_size(size: u64) -> String {
    if size < 1024 {
        return format!("{size} B");
    }
    let mut size: f64 = size as _;
    let prefixes = " kMGTPEZY";  // the " " is a placeholder
    let mut prefix_index: usize = 0;
    while prefix_index < prefixes.len() && size >= 1024.0 {
        prefix_index += 1;
        size /= 1024.0;
    }
    format!("{} {}iB", size, &prefixes[prefix_index..=prefix_index])
}

fn run_command_wrapper(args: &[impl AsRef<OsStr>], get_stdout: bool) -> Result<String, Box<dyn std::error::Error>> {
    let (program, args) = args.split_first().unwrap();
    let mut command = Command::new(program);
    command.args(args);
    if !get_stdout {
        command.stdout(Stdio::inherit());
    }
    command.stderr(Stdio::inherit());
    let result = command.output()?;
    if !result.status.success() {
        exit(result.status.code().unwrap_or(1));
    }
    Ok(String::from_utf8(result.stdout)?)
}

/// Run a command. If it command fails, the whole program exit with the same status code as it.
fn run_command(args: &[impl AsRef<OsStr>]) -> Result<(), Box<dyn std::error::Error>> {
    run_command_wrapper(args, false)?;
    Ok(())
}

/// Run a command and return its stdout as a `String`.
/// If the command fails, the whole program exit with the same status code as it.
fn get_command_output(args: &[impl AsRef<OsStr>]) -> Result<String, Box<dyn std::error::Error>> {
    run_command_wrapper(args, true)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_json = get_command_output(&["cargo", "read-manifest"])?;
    let manifest: Manifest = serde_json::from_str(manifest_json.as_str())?;
    let base_folder_str = manifest.manifest_path.parent().ok_or("could not get the project folder")?.as_os_str().to_str().unwrap();

    let targets = [
        ("windows", "x86_64-pc-windows-gnu"),
        ("mac", "aarch64-apple-darwin"),
        ("linux", "x86_64-unknown-linux-gnu"),
    ];

    eprintln!("Installing toolchains...");
    let mut command = vec!["rustup", "target", "add"];
    for (_, target) in targets {
        command.push(target);
    }
    run_command(&command)?;

    eprintln!("Building...");
    let mut command = vec!["cargo", "build", "--release", "--message-format", "json"];
    if std::env::args().any(| x | x == "--verbose") {
        command.push("--verbose");
    }
    for (_, target) in targets {
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

    let mut table = vec![];

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
            for (platform_name, platform) in targets {
                let text_to_match = executable.to_str().unwrap().replace(base_folder_str, "");
                if text_to_match.contains(platform) {
                    suffix = platform_name;
                    break;
                }
            }
            if suffix.is_empty() {
                eprintln!("WARNING: could not find suffix to add to executable");
            }
            eprintln!("Copying {:?} to {:?}...", executable, folder.as_os_str());
            let mut dest = Path::join(&folder, executable.file_name().unwrap());
            let file_name = format!(
                "{}-{}{}",
                dest.file_stem().unwrap_or_default().to_string_lossy(),
                suffix,
                dest.extension().map(| x | ".".to_owned() + x.to_str().unwrap_or("")).unwrap_or_default(),
            );
            dest.set_file_name(&file_name);
            copy(&executable, &dest)?;
            table.push((file_name, metadata(dest).unwrap().len()));
        }
    }

    let mut contents = String::from("# Latest build\n\n| File name | Size |\n| - | - |\n");
    for (file_name, file_size) in table {
        contents += format!("| [{file_name}](dist/{file_name}) | {} |\n", format_file_size(file_size)).as_str();
    }

    write("docs/src/latest-build.md", contents)?;

    eprintln!("Building book...");
    run_command(&["mdbook", "build", "docs"])?;

    Ok(())
}
