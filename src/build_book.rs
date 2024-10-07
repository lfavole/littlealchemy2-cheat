//! A command-line utility that builds the binaries and the book.
use std::{fs::{read_dir, remove_dir, remove_file, rename, write}, path::PathBuf};

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

mod run_command;
use run_command::run_command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Building technical docs...");
    run_command(&["cargo", "doc", "--no-deps"])?;

    eprintln!("Installing mdbook...");
    run_command(&["cargo", "install", "mdbook"])?;

    let folder = PathBuf::from("docs/src/dist");

    let mut table = vec![];

    for entry in read_dir(folder)? {
        let file = entry?;
        eprintln!("Entry: {file:?}");
        table.push((
            format!("{}", file.path().file_stem().unwrap().to_string_lossy()),
            file.metadata().unwrap().len(),
        ));
    }

    eprintln!("Writing latest-build.md...");
    let mut contents = String::from("# Latest build\n\n| File name | Size |\n| - | - |\n");
    for (file_name, file_size) in table {
        contents += format!("| [{file_name}](dist/{file_name}) | {} |\n", format_file_size(file_size)).as_str();
    }

    write("docs/src/latest-build.md", contents)?;

    eprintln!("Building book...");
    run_command(&["mdbook", "build", "docs"])?;

    eprintln!("Deleting dummy files...");
    remove_file("docs/book/doc/littlealchemy2_cheat/index.html")?;
    remove_dir("docs/book/doc/littlealchemy2_cheat")?;
    remove_dir("docs/book/doc")?;
    eprintln!("Moving technical docs...");
    rename("target/doc", "docs/book/doc")?;

    Ok(())
}
