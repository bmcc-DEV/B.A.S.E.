mod cli;
mod commands;
mod disasm;

fn main() {
    use clap::Parser;
    let cli = cli::Cli::parse();

    // Logging
    let level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(format!("base={},base_cli={}", level, level))
        .with_target(false)
        .init();

    match commands::execute(&cli.command, &cli.output) {
        Ok(()) => {}
        Err(e) => {
            tracing::error!("{}", e);
            std::process::exit(1);
        }
    }
}
