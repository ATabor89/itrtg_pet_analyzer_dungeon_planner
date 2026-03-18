mod parser;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pet-importer")]
#[command(about = "Import pet data from the ITRTG in-game export into structured YAML")]
struct Cli {
    /// Path to the pet stats export file (semicolon-delimited).
    /// If omitted, reads from the clipboard.
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Output file path (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let source = if let Some(path) = &cli.file {
        std::fs::read_to_string(path)?
    } else {
        eprintln!("Reading pet export from clipboard...");
        let mut clipboard = arboard::Clipboard::new()?;
        let text = clipboard.get_text()?;
        if !text.starts_with("Name;") {
            anyhow::bail!(
                "Clipboard doesn't appear to contain a pet stats export \
                 (expected header starting with \"Name;\"). \
                 Export your pets in-game first, or use --file."
            );
        }
        text
    };

    let pets = parser::parse_export(&source)?;

    let yaml = serde_yaml::to_string(&pets)?;

    if let Some(output_path) = &cli.output {
        std::fs::write(output_path, &yaml)?;
        eprintln!("Wrote {} pets to {}", pets.len(), output_path.display());
    } else {
        print!("{yaml}");
    }

    Ok(())
}
