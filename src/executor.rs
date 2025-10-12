use anyhow::{Context, Result};
use colored::*;
use std::process::{Command, Stdio};

/// Execute a shell command and return the result
pub fn execute_command(cmd: &str, working_dir: &str, verbose: bool) -> Result<()> {
    eprintln!("{} Running `{}`", "INFO:".cyan(), cmd);

    if verbose {
        eprintln!("{} Working directory: {}", "DEBUG:".blue(), working_dir);
        eprintln!("{} Command: {}", "DEBUG:".blue(), cmd);
    }

    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(working_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to execute command")?;

    if !status.success() {
        let exit_code = status
            .code()
            .map(|c| format!("code: {}", c))
            .unwrap_or_else(|| "unknown (terminated by signal)".to_string());
        anyhow::bail!("Command '{}' failed with exit {}", cmd, exit_code);
    }

    Ok(())
}

/// Execute multiple commands sequentially in a specific directory
/// Stops on first failure and provides clear error context
pub fn execute_commands_sequential(
    commands: &[String],
    working_dir: &str,
    verbose: bool,
) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }

    for (i, cmd) in commands.iter().enumerate() {
        eprintln!(
            "{} Step {}/{}: Running `{}`",
            "INFO:".cyan(),
            i + 1,
            commands.len(),
            cmd
        );

        if verbose {
            eprintln!("{} Working directory: {}", "DEBUG:".blue(), working_dir);
            eprintln!("{} Command: {}", "DEBUG:".blue(), cmd);
        }

        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(working_dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context(format!("Failed to execute command: {}", cmd))?;

        if !status.success() {
            let exit_code = status
                .code()
                .map(|c| format!("code: {}", c))
                .unwrap_or_else(|| "unknown (terminated by signal)".to_string());
            anyhow::bail!(
                "Command failed at step {}/{}: {}\nExit {}",
                i + 1,
                commands.len(),
                cmd,
                exit_code
            );
        }
    }

    Ok(())
}

/// Execute a command and capture its output, returning exit code and output
/// In verbose mode, streams output to terminal while still capturing exit code
pub fn execute_command_with_output(
    cmd: &str,
    working_dir: &str,
    verbose: bool,
) -> Result<(i32, Option<String>)> {
    if verbose {
        eprintln!("{} Running `{}`", "INFO:".cyan(), cmd);
        eprintln!("{} Working directory: {}", "DEBUG:".blue(), working_dir);
    }

    if verbose {
        // In verbose mode, stream output to terminal and just capture exit code
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(working_dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Failed to execute command")?;

        let exit_code = status.code().unwrap_or(-1);
        Ok((exit_code, None))
    } else {
        // In non-verbose mode, capture output silently
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(working_dir)
            .output()
            .context("Failed to execute command")?;

        let exit_code = output.status.code().unwrap_or(-1);
        let output_text = String::from_utf8_lossy(&output.stdout).to_string();

        Ok((exit_code, Some(output_text)))
    }
}

/// Execute a command and capture its output (legacy version that fails on error)
pub fn execute_command_output(cmd: &str, working_dir: &str, verbose: bool) -> Result<String> {
    if verbose {
        eprintln!("{} Running `{}` (capturing output)", "DEBUG:".blue(), cmd);
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(working_dir)
        .output()
        .context("Failed to execute command")?;

    if !output.status.success() {
        let exit_code = output
            .status
            .code()
            .map(|c| format!("code: {}", c))
            .unwrap_or_else(|| "unknown (terminated by signal)".to_string());
        anyhow::bail!(
            "Command '{}' failed with exit {}\nstderr: {}",
            cmd,
            exit_code,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Execute multiple commands sequentially with output capture support
/// Returns the exit code and optionally captured output from the last command
/// Does not automatically fail on non-zero exit codes - caller interprets them
pub fn execute_commands_sequential_with_output(
    commands: &[String],
    working_dir: &str,
    verbose: bool,
    capture_last: bool,
) -> Result<(i32, Option<String>)> {
    if commands.is_empty() {
        return Ok((0, None));
    }

    let total = commands.len();

    // Execute all commands except the last one
    for (i, cmd) in commands.iter().take(total.saturating_sub(1)).enumerate() {
        if verbose {
            eprintln!(
                "{} Step {}/{}: Running `{}`",
                "INFO:".cyan(),
                i + 1,
                total,
                cmd
            );
            eprintln!("{} Working directory: {}", "DEBUG:".blue(), working_dir);
            eprintln!("{} Command: {}", "DEBUG:".blue(), cmd);
        }

        let status = if verbose {
            // Stream output in verbose mode
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(working_dir)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .context(format!("Failed to execute command: {}", cmd))?
        } else {
            // Capture and suppress output in non-verbose mode
            let output = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(working_dir)
                .output()
                .context(format!("Failed to execute command: {}", cmd))?;
            output.status
        };

        if !status.success() {
            let exit_code = status
                .code()
                .map(|c| format!("code: {}", c))
                .unwrap_or_else(|| "unknown (terminated by signal)".to_string());
            anyhow::bail!(
                "Command failed at step {}/{}: {}\nExit {}",
                i + 1,
                total,
                cmd,
                exit_code
            );
        }
    }

    // Execute the last command with optional output capture
    if let Some(last_cmd) = commands.last() {
        if verbose {
            eprintln!(
                "{} Step {}/{}: Running `{}`",
                "INFO:".cyan(),
                total,
                total,
                last_cmd
            );
            eprintln!("{} Working directory: {}", "DEBUG:".blue(), working_dir);
            eprintln!("{} Command: {}", "DEBUG:".blue(), last_cmd);
        }

        if capture_last {
            // Capture output mode
            let output = Command::new("sh")
                .arg("-c")
                .arg(last_cmd)
                .current_dir(working_dir)
                .output()
                .context(format!("Failed to execute command: {}", last_cmd))?;

            let exit_code = output.status.code().unwrap_or(-1);
            let output_text = String::from_utf8_lossy(&output.stdout).to_string();

            Ok((exit_code, Some(output_text)))
        } else {
            // Streaming mode
            let status = Command::new("sh")
                .arg("-c")
                .arg(last_cmd)
                .current_dir(working_dir)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .context(format!("Failed to execute command: {}", last_cmd))?;

            let exit_code = status.code().unwrap_or(-1);
            Ok((exit_code, None))
        }
    } else {
        Ok((0, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_command_output() {
        let result = execute_command_output("echo 'test'", "./", false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "test");
    }

    #[test]
    fn test_execute_command_output_failure() {
        let result = execute_command_output("exit 1", "./", false);
        assert!(result.is_err());
    }
}
