use std::{ffi::OsStr, process::{exit, Command, Stdio}};

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
pub fn run_command(args: &[impl AsRef<OsStr>]) -> Result<(), Box<dyn std::error::Error>> {
    run_command_wrapper(args, false)?;
    Ok(())
}

/// Run a command and return its stdout as a `String`.
/// If the command fails, the whole program exit with the same status code as it.
pub fn get_command_output(args: &[impl AsRef<OsStr>]) -> Result<String, Box<dyn std::error::Error>> {
    run_command_wrapper(args, true)
}
