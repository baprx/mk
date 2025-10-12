use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "mk")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Disable gitignore filtering during directory walk
    #[arg(long, global = true)]
    pub no_ignore: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Apply infrastructure changes
    Apply {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Check/plan infrastructure changes without applying
    Check {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Show diff of infrastructure changes
    Diff {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Plan infrastructure changes (terraform alias for check)
    Plan {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Delete/destroy infrastructure
    Delete {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Destroy infrastructure (alias for delete)
    Destroy {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Uninstall infrastructure (alias for delete)
    Uninstall {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Update dependencies
    Deps {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Render templates locally
    Template {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Get terraform output value
    Output {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Output key name (omit or use --all to show all outputs)
        key: Option<String>,
        /// Show all outputs
        #[arg(short, long)]
        all: bool,
    },
    /// List ansible inventory
    List {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Additional options to pass to the underlying command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<String>,
    },
    /// Duplicate environment configuration
    Duplicate {
        /// Project path
        project_path: String,
        /// Source environment
        source_env: String,
        /// Target environment
        target_env: String,
    },
    /// Force unlock terraform state
    Unlock {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
        /// Lock ID
        lock_id: String,
    },
    /// Show the current terraform state
    Show {
        /// Project path
        project_path: String,
        /// Environment name
        environment: String,
    },
    /// Generate shell completions
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Initialize configuration file
    Init {
        /// Custom path for config file (default: ~/.config/mk/config.toml)
        #[arg(short, long)]
        path: Option<String>,
        /// Overwrite existing config file
        #[arg(short, long)]
        force: bool,
    },
    /// Hidden command for shell completion: list available environments
    #[command(hide = true)]
    CompleteEnv {
        /// Project path to detect environments from
        project_path: String,
    },
    /// Hidden command for shell completion: list available terraform output keys
    #[command(hide = true)]
    CompleteOutputKey {
        /// Project path to scan for terraform outputs
        project_path: String,
    },
    /// Check for dependency updates and apply them interactively
    Bump {
        /// Project path
        project_path: String,
        /// Include pre-release versions (alpha, beta, rc, etc.)
        #[arg(long)]
        include_prereleases: bool,
        /// Recursively scan subdirectories for projects
        #[arg(short, long)]
        recursive: bool,
    },
    /// Check for IaC drift across multiple stacks
    Drift {
        /// Base directory to scan recursively for IaC projects
        base_path: String,
        /// Show verbose output including terraform/helm plan details
        #[arg(short, long)]
        verbose: bool,
        /// Filter to only check specific technology (terraform or helm)
        #[arg(short = 't', long)]
        tech: Option<String>,
        /// Check only specific environment(s) (can be repeated)
        #[arg(short = 'e', long = "env")]
        environments: Vec<String>,
        /// Capture full output to log files in .drift-logs/ directory
        #[arg(short, long)]
        capture: bool,
        /// Maximum depth for recursive scanning (default: 5)
        #[arg(short = 'd', long, default_value = "5")]
        max_depth: usize,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
        }
    }
}
