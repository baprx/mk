use super::Action;
use anyhow::Result;

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

    let cmd = match action {
        Action::Apply => {
            format!(
                "kustomize build {}/overlays/{} | kubectl apply --validate=false -f -{}",
                project_path, environment, opts
            )
        }
        Action::Check | Action::Diff => {
            format!(
                "kustomize build {}/overlays/{} | kubectl diff -f - || test $? -eq 1",
                project_path, environment
            )
        }
        Action::Template => {
            format!(
                "kustomize build {}/overlays/{}{}",
                project_path, environment, opts
            )
        }
        Action::Delete | Action::Destroy | Action::Uninstall => {
            format!(
                "kustomize build {}/overlays/{} | kubectl delete -f -{}",
                project_path, environment, opts
            )
        }
        _ => {
            anyhow::bail!("Action {:?} not implemented for kustomize", action);
        }
    };

    Ok(cmd)
}
