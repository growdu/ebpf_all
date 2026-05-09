mod commands;

use clap::{Parser, Subcommand};

const ABOUT: &str = "
UOF CLI — Universal Observability Framework control plane client.

Manage agents, plugins, templates, and template bindings via the
Control Plane HTTP API.

Examples:

  # List all plugins
  uof-cli plugin list

  # Create a new plugin
  uof-cli plugin create postgres-observability template uof

  # List all templates
  uof-cli template list

  # Create a template
  uof-cli template create \\
    --plugin-id <uuid> \\
    --name postgres-dba \\
    --version 0.1.0 \\
    --target-software postgresql

  # Control plane health check
  uof-cli agent health
";

#[derive(Debug, Parser)]
#[command(name = "uof")]
#[command(about = "UOF control plane CLI", long_about = ABOUT)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Control Plane API base URL.
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    url: String,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Agent operations.
    Agent {
        #[command(subcommand)]
        sub: commands::AgentCommands,
    },
    /// Plugin operations.
    Plugin {
        #[command(subcommand)]
        sub: commands::PluginCommands,
    },
    /// Template operations.
    Template {
        #[command(subcommand)]
        sub: commands::TemplateCommands,
    },
    /// Template binding operations.
    Binding {
        #[command(subcommand)]
        sub: commands::BindingCommands,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build HTTP client");
    let base_url = cli.url.trim_end_matches('/');

    match cli.command {
        Some(Commands::Agent { sub }) => commands::handle_agent(&client, base_url, sub).await,
        Some(Commands::Plugin { sub }) => commands::handle_plugin(&client, base_url, sub).await,
        Some(Commands::Template { sub }) => commands::handle_template(&client, base_url, sub).await,
        Some(Commands::Binding { sub }) => commands::handle_binding(&client, base_url, sub).await,
        None => {
            println!("uof-cli {}", env!("CARGO_PKG_VERSION"));
            println!("Use --help for usage information.");
            Ok(())
        }
    }
}
