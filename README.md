# mk - Infrastructure Deployment CLI Tool

A CLI tool written in Rust for managing infrastructure deployments across multiple technologies (Terraform, Helm, Kustomize, and Ansible).

## Features

- 🔍 **Auto-detection** - Automatically detects technology type (terraform/helm/kustomize/ansible)
- ✅ **Environment validation** - Validates environment names before execution
- 🔒 **Kubernetes context safety** - Prevents deploying to wrong clusters (Helm/Kustomize)
- 🎨 **Color-coded output** - Clear, readable terminal output
- 🚀 **Direct command execution** - No intermediate make calls
- 🔧 **Special commands** - Support for `unlock`, and `duplicate`
- 🐚 **Shell completions** - Bash and Zsh support

## Installation

### From Source

```bash
cargo build --release --locked
cp target/release/mk ~/.local/bin/
```

### Shell Completions

Generate completions for your shell:

```bash
# Bash
mk completions bash > ~/.local/share/bash-completion/completions/mk
source ~/.local/share/bash-completion/completions/mk

# Zsh
source <(mk completions zsh)
```

The completions include **dynamic environment completion** based on your project structure:
- For Terraform projects: Completes from `tfvars/*.tfvars` files
- For Helm projects: Completes from `values/*/` directories
- For Kustomize projects: Completes from `overlays/*/` directories
- For Ansible projects: Completes from `inventories/*.yml` files

Example usage:
```bash
mk apply infrastructure/iam/terraform <TAB>
# Suggests: dev prod staging (based on actual tfvars files)
```

## Usage

### Basic Commands

```bash
# Apply infrastructure changes
mk apply <project-path> <environment> [options]

# Plan/check changes
mk check <project-path> <environment> [options]
mk plan <project-path> <environment> [options]  # Alias for check

# Show diff
mk diff <project-path> <environment> [options]

# Destroy/delete infrastructure
mk delete <project-path> <environment> [options]
mk destroy <project-path> <environment> [options]  # Alias
mk uninstall <project-path> <environment> [options]  # Alias

# Update dependencies
mk deps <project-path> <environment> [options]

# Render templates
mk template <project-path> <environment> [options]

# Get terraform output
mk output <project-path> <environment> <key> # Autocompletion works for <key>, also a --all flag is available

# List ansible inventory
mk list <project-path> <environment> [options]

# Duplicate environment
mk duplicate <project-path> <source-env> <target-env>

# Check for dependency updates (Terraform & Helm)
mk bump <project-path> [--include-prereleases] [--recursive]
```

### Special Commands

```bash
# Force unlock terraform state
mk unlock <project-path> <environment> <lock-id>
```

### Dependency Management

The `bump` command helps you keep Terraform modules and Helm charts up to date:

```bash
# Scan a single project for outdated dependencies
mk bump infrastructure/terraform/my-project

# Scan recursively across multiple projects (max depth: 5)
mk bump infrastructure --recursive

# Include pre-release versions (alpha, beta, rc)
mk bump infrastructure --include-prereleases --recursive

# Show verbose output during scanning
mk bump infrastructure --verbose --recursive
```

**Features:**
- Interactive selection of dependencies to update
- Support for Terraform registry modules and Helm chart repositories
- Caching of version lookups to avoid redundant API calls when scanning recursively
- Respects `.gitignore` patterns when scanning recursively
- Configurable maximum scan depth via `~/.config/mk/config.toml`

**Configuration:**

Create or update `~/.config/mk/config.toml`:

```toml
[bump]
# Maximum directory depth for recursive scanning (default: 5)
max_depth = 5
```

**Example output:**

```bash
$ mk bump infrastructure/pubsub --recursive
INFO: Scanning recursively (max depth: 5): infrastructure/pubsub
INFO: Found 3 Terraform project(s), 0 Helm project(s)
INFO: Found 2 dependencies with updates available

Select dependencies to update (Space to select, Enter to confirm):
> [x] terraform-google-modules/pubsub/google (pubsub.tf:3) 8.2.0 → 8.3.2
  [ ] terraform-google-modules/iam/google//modules/pubsub_topics_iam (pubsub.tf:15) 8.0.0 → 8.1.0

INFO: Updating selected dependencies...
  ✓ Updated terraform-google-modules/pubsub/google in pubsub.tf

SUCCESS: 1 dependencies updated across 3 project(s)
```

### Global Options

```bash
-v, --verbose    Enable verbose output
-h, --help      Show help information
-V, --version   Show version information
```

## Technology Detection

The tool automatically detects the technology based on project structure:

- **Terraform**: Directory named `terraform` or contains `tfvars/` directory
- **Helm**: Contains `values.yaml` file
- **Kustomize**: Contains `overlays/` directory
- **Ansible**: Directory named `ansible` or contains `inventories/` directory

## Environment Detection

Environments are detected from:

- **Terraform**: Files in `tfvars/` directory (e.g., `tfvars/dev.tfvars` → `dev`)
- **Helm**: Directories in `values/` (e.g., `values/dev/` → `dev`)
- **Kustomize**: Directories in `overlays/` (e.g., `overlays/dev/` → `dev`)
- **Ansible**: Files in `inventories/` (e.g., `inventories/dev.yml` → `dev`)

## Examples

### Terraform

```bash
# Plan infrastructure changes
mk plan infrastructure/iam/terraform demo-env

# Apply changes
mk apply infrastructure/iam/terraform demo-env

# Get output value
mk output infrastructure/iam/terraform demo-env vpc_id

# Duplicate environment
mk duplicate infrastructure/iam/terraform demo-env staging-env

# Unlock state
mk unlock infrastructure/iam/terraform demo-env 196787809097
```

### Helm

```bash
# Update dependencies
mk deps manifests/monitoring/prometheus demo-env

# Show diff
mk diff manifests/monitoring/prometheus demo-env

# Install/upgrade
mk apply manifests/monitoring/prometheus demo-env

# Uninstall
mk uninstall manifests/monitoring/prometheus demo-env

# Render templates
mk template manifests/monitoring/prometheus demo-env
```

### Kustomize

```bash
# Build manifests
mk template manifests/monitoring/stackdriver-exporter demo-env

# Show diff
mk diff manifests/monitoring/stackdriver-exporter demo-env

# Apply
mk apply manifests/monitoring/stackdriver-exporter demo-env

# Delete
mk delete manifests/monitoring/stackdriver-exporter demo-env
```

### Ansible

```bash
# Install dependencies
mk deps infrastructure/sftp/instance/ansible demo-env

# Check playbook (dry-run)
mk check infrastructure/sftp/instance/ansible demo-env

# Run playbook
mk apply infrastructure/sftp/instance/ansible demo-env

# List inventory
mk list infrastructure/sftp/instance/ansible demo-env
```

## Command Mapping

### Terraform

> Terraform init is automatically run before `apply`, `plan`, `destroy`, and `output` commands.

| Action       | Command                                           |
| ------------ | ------------------------------------------------- |
| `apply`      | `terraform apply -var-file=tfvars/{env}.tfvars`   |
| `plan/check` | `terraform plan -var-file=tfvars/{env}.tfvars`    |
| `destroy`    | `terraform destroy -var-file=tfvars/{env}.tfvars` |
| `output`     | `terraform output {key}`                          |

### Helm

> Helm dependencies are automatically updated before `apply`, `diff`, and `template` if needed (based on `Chart.lock` & `charts/*.tgz`).

| Action      | Command                                  |
| ----------- | ---------------------------------------- |
| `apply`     | `helmfile sync -e {env} --skip-deps`     |
| `diff`      | `helmfile diff -e {env} --skip-deps`     |
| `deps`      | `helmfile deps -e {env}`                 |
| `template`  | `helmfile template -e {env} --skip-deps` |
| `uninstall` | `helmfile destroy -e {env} --skip-deps`  |

### Kustomize

| Action     | Command                                                 |
| ---------- | ------------------------------------------------------- |
| `apply`    | `kustomize build overlays/{env} \| kubectl apply -f -`  |
| `diff`     | `kustomize build overlays/{env} \| kubectl diff -f -`   |
| `template` | `kustomize build overlays/{env}`                        |
| `delete`   | `kustomize build overlays/{env} \| kubectl delete -f -` |

### Ansible

| Action       | Command                                                      |
| ------------ | ------------------------------------------------------------ |
| `apply`      | `ansible-playbook -i inventories/{env}.yml playbook.yml -D`  |
| `check/diff` | `ansible-playbook -i inventories/{env}.yml playbook.yml -DC` |
| `deps`       | `ansible-galaxy install -r roles/requirements.yml -f`        |
| `list`       | `ansible-inventory -i inventories/{env}.yml --list \| jq .`  |

## Configuration

### Configuration File

Initialize a configuration file with examples:

```bash
mk init
```

This creates `~/.config/mk/config.toml` with the following options:

#### Technology Priority

Set the priority order when multiple technologies are detected:

```toml
# Prioritize Terraform over other technologies
technology_priority = ["terraform", "kustomize", "helm", "ansible"]
```

#### Bump Configuration

Configure the dependency bump command:

```toml
[bump]
# Maximum directory depth for recursive scanning (default: 5)
max_depth = 5
```

#### Kubernetes Context Safety (Helm/Kustomize)

Automatically validates that you're using the correct kubectl context before applying or diffing changes:

```toml
[context]
# Disable context validation checks (default: false)
disable_context_check = false

# Context mappings: repository -> environment -> kubectl context
# Example:
[context.mappings."github.com/user/infra"]
prod = "gke_project_cluster-prod"
staging = "gke_project_cluster-staging"
```

**How it works:**

1. When you run `mk apply` or `mk diff` for Helm/Kustomize:
   - The tool checks if a context mapping exists for your git repository and environment
   - If a mapping exists, it verifies your current kubectl context matches the expected one
   - If no mapping exists, it prompts you to save the current context for future use

2. **Example workflow:**

```bash
# First time running for a new environment
$ mk apply ./my-helm-chart prod

WARNING: No Kubernetes context configured for:
  Repository: github.com/user/infra
  Environment: prod

  Current kubectl context: gke_project_cluster-prod

Continue and save this context for future use? [Y/n]: y
SUCCESS: Context mapping saved to ~/.config/mk/config.toml

# Future runs validate automatically
$ mk apply ./my-helm-chart prod
✓ Context validated: gke_project_cluster-prod

# If you accidentally use wrong context
$ kubectl config use-context gke_project_cluster-staging
$ mk apply ./my-helm-chart prod

ERROR: Kubernetes context mismatch!
Repository: github.com/user/infra
Environment: prod
Expected context: gke_project_cluster-prod
Current context: gke_project_cluster-staging

Please switch to the correct context with:
kubectl config use-context gke_project_cluster-prod
```

3. **Team sharing:**

You can also create a `.mk/contexts.toml` file in your git repository to share context mappings with your team:

```toml
[mappings."github.com/user/infra"]
prod = "gke_project_cluster-prod"
staging = "gke_project_cluster-staging"
dev = "gke_project_cluster-dev"
```

The tool prioritizes repo-level configs over user-level configs, allowing teams to establish consistent context mappings.

## Development

### Building

```bash
cargo build --release --locked
# or continuously with
watchexec -r -e rs --bell -- 'cargo build --release --locked'
```

### Testing

```bash
cargo test
```

### Linting

```bash
cargo clippy
```

## License

MIT License. See [LICENSE](LICENSE) for details.
