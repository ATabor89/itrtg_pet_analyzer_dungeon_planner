mod models;
mod parser;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pet_extractor")]
#[command(about = "Extract pet data from the ITRTG Wiki into structured YAML")]
struct Cli {
    /// Path to a local wiki source file (mediawiki markup)
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Fetch the latest source directly from the wiki
    #[arg(short = 'w', long)]
    fetch_wiki: bool,

    /// Output file path (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let source = if cli.fetch_wiki {
        eprintln!("Fetching latest wiki source...");
        let url = "https://itrtg.wiki.gg/wiki/Pets?action=raw";
        let client = reqwest::blocking::Client::builder()
            .user_agent("pet_extractor/0.1.0 (ITRTG tool)")
            .build()?;
        let resp = client.get(url).send()?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to fetch wiki: HTTP {}", resp.status());
        }
        resp.text()?
    } else if let Some(path) = &cli.file {
        std::fs::read_to_string(path)?
    } else {
        anyhow::bail!("Provide either --file <path> or --fetch-wiki");
    };

    let pets = parser::parse_pets(&source)?;

    let yaml = serde_yaml::to_string(&pets)?;

    if let Some(output_path) = &cli.output {
        std::fs::write(output_path, &yaml)?;
        eprintln!("Wrote {} pets to {}", pets.len(), output_path.display());
    } else {
        print!("{yaml}");
    }

    Ok(())
}
