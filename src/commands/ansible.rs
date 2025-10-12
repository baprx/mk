use super::Action;
use anyhow::Result;
use std::process::Command;

pub fn get_command(
    action: &Action,
    project_path: &str,
    environment: &str,
    options: &[String],
) -> Result<String> {
    let options_str = options.join(" ");
    let opts = if options_str.is_empty() {
        String::new()
    } else {
        format!(" {}", options_str)
    };

    // Find the inventory file (could be .yml or .yaml)
    let inventory_pattern = format!("inventories/{}.*yml", environment);

    let cmd = match action {
        Action::Apply => {
            format!(
                "ansible-playbook -i {} playbook.yml -D{}",
                inventory_pattern, opts
            )
        }
        Action::Check | Action::Diff => {
            format!(
                "ansible-playbook -i {} playbook.yml -DC{}",
                inventory_pattern, opts
            )
        }
        Action::Deps => {
            format!(
                "ansible-galaxy install -r roles/requirements.yml -f{}",
                opts
            )
        }
        Action::List => {
            // Run ansible-inventory and parse JSON output natively
            list_ansible_inventory(project_path, &inventory_pattern, options)?;
            return Ok(String::new());
        }
        _ => {
            anyhow::bail!("Action {:?} not implemented for ansible", action);
        }
    };

    Ok(cmd)
}

/// List ansible inventory with pretty-printed JSON output
fn list_ansible_inventory(
    project_path: &str,
    inventory_pattern: &str,
    options: &[String],
) -> Result<()> {
    let options_str = options.join(" ");
    let opts = if options_str.is_empty() {
        String::new()
    } else {
        format!(" {}", options_str)
    };

    // Run ansible-inventory command
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ansible-inventory -i {} --list{}",
            inventory_pattern, opts
        ))
        .current_dir(project_path)
        .output()?;

    if output.status.success() {
        // Parse JSON and pretty-print it
        let json_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
            let pretty = serde_json::to_string_pretty(&parsed)?;
            println!("{}", pretty);
        } else {
            // If parsing fails, just print the raw output
            print!("{}", json_str);
        }
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ansible-inventory failed: {}", error);
    }

    Ok(())
}
