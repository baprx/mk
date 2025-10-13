use super::Action;
use anyhow::Result;

pub fn get_command(action: &Action, environment: &str, options: &[String]) -> Result<String> {
    let options_str = options.join(" ");
    let opts = if options_str.is_empty() {
        String::new()
    } else {
        format!(" {}", options_str)
    };

    let cmd = match action {
        Action::Apply => {
            format!(
                "kustomize build overlays/{} | kubectl apply {} -f -",
                environment, opts
            )
        }
        Action::Check | Action::Diff => {
            format!(
                "kustomize build overlays/{} | kubectl diff {} -f - || test $? -eq 1",
                environment, opts
            )
        }
        Action::Template => {
            format!("kustomize build overlays/{} {}", environment, opts)
        }
        Action::Delete | Action::Destroy | Action::Uninstall => {
            format!(
                "kustomize build overlays/{} | kubectl delete {} -f -",
                environment, opts
            )
        }
        _ => {
            anyhow::bail!("Action {:?} not implemented for kustomize", action);
        }
    };

    Ok(cmd)
}
