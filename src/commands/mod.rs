pub mod ansible;
pub mod helm;
pub mod kustomize;
pub mod terraform;

use crate::techno::Technology;
use anyhow::Result;

#[derive(Debug)]
pub enum Action {
    Apply,
    Check,
    Diff,
    Plan,
    Delete,
    Destroy,
    Uninstall,
    Deps,
    Template,
    Output { key: Option<String> },
    List,
    Duplicate { target_env: String },
    Unlock { lock_id: String },
    Show,
}

/// Get the command(s) to execute based on the action, technology, and parameters
/// Returns a vector of commands for technologies that support sequential execution (e.g., Terraform)
/// or a single-item vector for technologies using shell chaining
pub fn get_command(
    action: &Action,
    project_path: &str,
    environment: &str,
    techno: Technology,
    options: &[String],
    verbose: bool,
    silent: bool,
) -> Result<Vec<String>> {
    match techno {
        Technology::Terraform => terraform::get_command(action, project_path, environment, options),
        Technology::Helm => {
            let cmd =
                helm::get_command(action, project_path, environment, options, verbose, silent)?;
            Ok(cmd.map(|c| vec![c]).unwrap_or_else(Vec::new))
        }
        Technology::Kustomize => {
            let cmd = kustomize::get_command(action, environment, options)?;
            Ok(vec![cmd])
        }
        Technology::Ansible => {
            let cmd = ansible::get_command(action, project_path, environment, options)?;
            Ok(vec![cmd])
        }
    }
}
