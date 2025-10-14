mod bump;
mod cli;
mod commands;
mod config;
mod context;
mod drift;
mod env;
mod executor;
mod techno;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell as CompletionShell};
use colored::*;

use cli::{Cli, Commands, Shell};
use commands::Action;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "ERROR:".red(), e);
        // Print the full error chain
        let mut source = e.source();
        while let Some(err) = source {
            eprintln!("  Caused by: {}", err);
            source = err.source();
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path, force } => init_config(path, force),
        Commands::Completions { shell } => {
            generate_completions(shell);
            Ok(())
        }
        Commands::CompleteEnv { project_path } => complete_env(&project_path),
        Commands::CompleteOutputKey { project_path } => complete_output_key(&project_path),
        Commands::Unlock {
            project_path,
            environment,
            lock_id,
        } => execute_action(
            Action::Unlock { lock_id },
            &project_path,
            &environment,
            &[],
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Show {
            project_path,
            environment,
        } => execute_action(
            Action::Show,
            &project_path,
            &environment,
            &[],
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Apply {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Apply,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Check {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Check,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Diff {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Diff,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Plan {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Plan,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Delete {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Delete,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Destroy {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Destroy,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Uninstall {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Uninstall,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Deps {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Deps,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Template {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::Template,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Output {
            project_path,
            environment,
            key,
            all,
        } => {
            let output_key = if all { None } else { key };
            execute_action(
                Action::Output { key: output_key },
                &project_path,
                &environment,
                &[],
                cli.verbose,
                cli.no_ignore,
            )
        }
        Commands::List {
            project_path,
            environment,
            options,
        } => execute_action(
            Action::List,
            &project_path,
            &environment,
            &options,
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Duplicate {
            project_path,
            source_env,
            target_env,
        } => execute_action(
            Action::Duplicate { target_env },
            &project_path,
            &source_env,
            &[],
            cli.verbose,
            cli.no_ignore,
        ),
        Commands::Bump {
            project_path,
            include_prereleases,
            recursive,
        } => bump::run_bump(
            &project_path,
            cli.verbose,
            include_prereleases,
            recursive,
            cli.no_ignore,
        ),
        Commands::Drift {
            base_path,
            verbose,
            tech,
            environments,
            capture,
            max_depth,
        } => drift::run_drift(
            &base_path,
            verbose,
            tech,
            environments,
            capture,
            max_depth,
            cli.no_ignore,
        ),
    }
}

fn execute_action(
    action: Action,
    project_path: &str,
    environment: &str,
    options: &[String],
    verbose: bool,
    no_ignore: bool,
) -> Result<()> {
    execute_action_internal(
        action,
        project_path,
        environment,
        options,
        verbose,
        false,
        no_ignore,
    )?;
    Ok(())
}

/// Execute an action with optional drift mode
/// Returns (exit_code, output) when in drift mode, otherwise just executes normally
pub fn execute_action_internal(
    action: Action,
    project_path: &str,
    environment: &str,
    options: &[String],
    verbose: bool,
    drift_mode: bool,
    no_ignore: bool,
) -> Result<(i32, Option<String>)> {
    // Detect technology and get the actual path where it was found
    let (techno, actual_path) = techno::detect_technology(project_path, Some(&action), drift_mode)
        .context("Failed to detect technology")?;

    // Check environment validity (skip for deps action)
    // Use actual_path instead of project_path
    if !matches!(action, Action::Deps) {
        env::check_environment(&actual_path, environment, techno, no_ignore)
            .context("Invalid environment")?;
    }

    if matches!(
        techno,
        techno::Technology::Helm | techno::Technology::Kustomize
    ) && matches!(
        action,
        Action::Apply
            | Action::Diff
            | Action::Check
            | Action::Delete
            | Action::Destroy
            | Action::Uninstall
    ) {
        context::validate_context(&actual_path, environment, verbose)
            .context("Kubernetes context validation failed")?;
    }

    // Get the commands to execute
    // Use actual_path instead of project_path
    let commands = commands::get_command(
        &action,
        &actual_path,
        environment,
        techno,
        options,
        verbose,
        drift_mode,
    )
    .context("Failed to generate commands")?;

    // Execute the commands
    if commands.is_empty() {
        // No commands to execute (e.g., for Duplicate action that does native operations)
        return Ok((0, None));
    }

    if drift_mode {
        // In drift mode, capture output and return exit code
        let (exit_code, output) = if commands.len() == 1 {
            executor::execute_command_with_output(&commands[0], &actual_path, verbose)?
        } else {
            executor::execute_commands_sequential_with_output(
                &commands,
                &actual_path,
                verbose,
                true,
            )?
        };
        Ok((exit_code, output))
    } else {
        // Normal mode - just execute
        if commands.len() == 1 {
            executor::execute_command(&commands[0], &actual_path, verbose)
                .context("Failed to execute command")?;
        } else {
            executor::execute_commands_sequential(&commands, &actual_path, verbose)
                .context("Failed to execute commands")?;
        }
        Ok((0, None))
    }
}

fn init_config(path: Option<String>, force: bool) -> Result<()> {
    let path_buf = path.map(std::path::PathBuf::from);

    match config::Config::init_config(path_buf, force) {
        Ok(config_path) => {
            eprintln!(
                "{} Configuration file created at: {}",
                "SUCCESS:".green(),
                config_path.display()
            );
            eprintln!(
                "{} Edit the file to customize mk's behavior",
                "INFO:".cyan()
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn complete_env(project_path: &str) -> Result<()> {
    // Silently detect technology and get environments
    // This is used by shell completion, so we only output environment names
    // Pass silent=true to suppress all INFO messages
    if let Ok((techno, actual_path)) = techno::detect_technology(project_path, None, true) {
        if let Ok(envs) = env::get_environments(&actual_path, techno, false) {
            // Print each environment on a separate line for shell completion
            for env in envs {
                println!("{}", env);
            }
        }
    }

    Ok(())
}

fn complete_output_key(project_path: &str) -> Result<()> {
    use ignore::WalkBuilder;
    use regex::Regex;
    use std::collections::HashSet;
    use std::fs;

    // Scan for .tf files and extract output keys
    let output_regex = Regex::new(r#"^output\s+"([^"]+)"\s+\{"#).unwrap();
    let mut output_keys = HashSet::new();

    // Walk through the project directory looking for .tf files
    for entry in WalkBuilder::new(project_path)
        .max_depth(Some(3))
        .build()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            if let Some(ext) = entry.path().extension() {
                if ext == "tf" {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        for line in content.lines() {
                            if let Some(captures) = output_regex.captures(line) {
                                if let Some(key) = captures.get(1) {
                                    output_keys.insert(key.as_str().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Print each output key on a separate line for shell completion
    for key in output_keys {
        println!("{}", key);
    }

    Ok(())
}

fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let shell_type = match shell {
        Shell::Bash => CompletionShell::Bash,
        Shell::Zsh => CompletionShell::Zsh,
    };

    eprintln!("Generating completion file for {}...", shell);

    // Generate base completions to a string buffer
    let mut buf = Vec::new();
    generate(shell_type, &mut cmd, "mk", &mut buf);
    let base_completions = String::from_utf8_lossy(&buf);

    // Add custom completion logic for environment and output keys
    match shell {
        Shell::Bash => {
            print!("{}", base_completions);
            print!("{}", get_bash_dynamic_completion_wrapper());
        }
        Shell::Zsh => {
            // For Zsh, inject the custom completion helpers
            print!("{}", get_zsh_dynamic_completion_helpers());

            // Post-process the base completions to integrate custom completions
            let processed = base_completions
                .replace(
                    ":environment -- Environment name:_default",
                    ":environment -- Environment name:_mk_environments",
                )
                .replace(
                    ":source_env -- Source environment:_default",
                    ":source_env -- Source environment:_mk_environments",
                )
                .replace(
                    ":key -- Output key name (omit or use --all to show all outputs):_default",
                    ":key -- Output key name (omit or use --all to show all outputs):_mk_output_keys",
                );

            print!("{}", processed);
        }
    }
}

fn get_zsh_dynamic_completion_helpers() -> &'static str {
    r#"#compdef mk

# Custom completion helpers
_mk_environments() {
    # In zsh completion context, use line array to get the project path
    # The project path is the first positional argument after the subcommand
    local project_path="${line[1]}"
    if [[ -n "$project_path" ]]; then
        local -a envs
        envs=($(mk complete-env "$project_path" 2>/dev/null))
        _describe 'environment' envs
    fi
}

_mk_output_keys() {
    local project_path="${line[1]}"
    if [[ -n "$project_path" ]]; then
        local -a keys
        keys=($(mk complete-output-key "$project_path" 2>/dev/null))
        _describe 'output key' keys
    fi
}

"#
}

fn get_bash_dynamic_completion_wrapper() -> &'static str {
    r#"
# Custom completion helpers for mk
_mk_environments() {
    # Extract the project path (first positional argument after subcommand)
    local project_path="${COMP_WORDS[2]}"
    if [[ -n "$project_path" ]]; then
        local envs=$(mk complete-env "$project_path" 2>/dev/null)
        COMPREPLY=($(compgen -W "$envs" -- "$cur"))
    fi
}

_mk_output_keys() {
    local project_path="${COMP_WORDS[2]}"
    if [[ -n "$project_path" ]]; then
        local keys=$(mk complete-output-key "$project_path" 2>/dev/null)
        COMPREPLY=($(compgen -W "$keys" -- "$cur"))
    fi
}

# Enhance the generated completion function
_mk_dynamic() {
    local cur prev words cword
    _init_completion || return

    # Determine which subcommand we're in
    local cmd=""
    local i
    for i in "${COMP_WORDS[@]:1:COMP_CWORD-1}"; do
        case "$i" in
            apply|check|diff|plan|delete|destroy|uninstall|deps|template|output|list|show|unlock|duplicate)
                cmd="$i"
                break
                ;;
        esac
    done

    case "$cmd" in
        apply|check|diff|plan|delete|destroy|uninstall|deps|template|list|show|unlock|duplicate)
            # These commands take: PROJECT_PATH ENVIRONMENT [OPTIONS]...
            if [[ ${COMP_CWORD} -eq 3 ]]; then
                _mk_environments
                return 0
            fi
            ;;
        output)
            # Output command takes: PROJECT_PATH ENVIRONMENT [KEY]
            if [[ ${COMP_CWORD} -eq 3 ]]; then
                _mk_environments
                return 0
            elif [[ ${COMP_CWORD} -eq 4 ]]; then
                _mk_output_keys
                return 0
            fi
            ;;
    esac

    # Fall back to base completion
    return 1
}

# Wrap the generated completion function
_mk_original=$(declare -f _mk)
eval "${_mk_original/_mk/_mk_base}"

_mk() {
    # Try dynamic completion first
    _mk_dynamic "$@" && return 0

    # Fall back to base completion
    _mk_base "$@"
}
"#
}
