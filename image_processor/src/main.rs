use clap::Parser;
use log::{error, info};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    input: PathBuf,
    output: PathBuf,
    plugin: String,
    params: PathBuf,

    #[arg(short, long)]
    plugin_path: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let start_time = Instant::now();
    let cli = Cli::parse();

    // Инициализация логирования
    image_processor::plugin_loader::init_logging(cli.verbose);

    // Определение пути к плагинам
    let plugin_path = match cli.plugin_path {
        Some(path) => path,
        None => {
            if cfg!(debug_assertions) {
                PathBuf::from("target/debug")
            } else {
                PathBuf::from("target/release")
            }
        }
    };

    info!("Input: {}", cli.input.display());
    info!("Output: {}", cli.output.display());
    info!("Plugin: {}", cli.plugin);
    info!("Params: {}", cli.params.display());
    info!("Plugin path: {}", plugin_path.display());

    match image_processor::process_image(
        &cli.input,
        &cli.output,
        &cli.plugin,
        &cli.params,
        &plugin_path,
    ) {
        Ok(_) => {
            let duration = start_time.elapsed();
            info!("Success! Processing time: {:?}", duration);
        }
        Err(e) => {
            error!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
