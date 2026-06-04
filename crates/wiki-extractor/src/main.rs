use clap::Parser;
use wiki_extractor::parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wiki-extractor")]
#[command(about = "Extract pet data from the ITRTG Wiki into structured YAML")]
struct Cli {
    /// Path to a local wiki source file (mediawiki markup).
    /// If omitted, fetches the latest source directly from the wiki.
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Output file path (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Skip the per-pet crawl that fills in evolution requirements. Faster, but
    /// leaves `evo_requirements` empty. The crawl runs by default (and in CI).
    #[arg(long)]
    skip_evo: bool,

    /// Milliseconds to wait between per-pet requests during the evo crawl, to
    /// stay polite to the wiki.
    #[arg(long, default_value_t = 300)]
    evo_delay_ms: u64,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let source = if let Some(path) = &cli.file {
        std::fs::read_to_string(path)?
    } else {
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
    };

    let mut pets = parser::parse_pets(&source)?;

    if !cli.skip_evo {
        crawl_evo_requirements(&mut pets, cli.evo_delay_ms)?;
    }

    let yaml = serde_yaml::to_string(&pets)?;

    if let Some(output_path) = &cli.output {
        std::fs::write(output_path, &yaml)?;
        eprintln!("Wrote {} pets to {}", pets.len(), output_path.display());
    } else {
        print!("{yaml}");
    }

    Ok(())
}

/// Visit each pet's rendered wiki page and fill in its evolution requirements.
///
/// This is the slow path (one request per pet), so it only runs in the
/// extractor/CI — never in the app, which reads the baked-in YAML. Failures for
/// individual pets are logged and skipped: a missing page or block just leaves
/// `evo_requirements` as `None` rather than aborting the whole run.
fn crawl_evo_requirements(pets: &mut [itrtg_models::WikiPet], delay_ms: u64) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("pet_extractor/0.1.0 (ITRTG tool)")
        .build()?;

    let total = pets.len();
    let mut found = 0usize;
    eprintln!("Crawling evolution requirements for {total} pets...");

    for (i, pet) in pets.iter_mut().enumerate() {
        // Be polite: space out requests so we don't hammer the wiki.
        if i > 0 && delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }

        match client.get(&pet.wiki_url).send() {
            Ok(resp) if resp.status().is_success() => match resp.text() {
                Ok(html) => {
                    if let Some(evo) = parser::parse_evo_requirements(&html) {
                        pet.evo_requirements = Some(evo);
                        found += 1;
                    } else {
                        eprintln!("  [{}/{total}] {}: no evolution block found", i + 1, pet.name);
                    }
                }
                Err(e) => eprintln!("  [{}/{total}] {}: read error: {e}", i + 1, pet.name),
            },
            Ok(resp) => {
                eprintln!("  [{}/{total}] {}: HTTP {}", i + 1, pet.name, resp.status())
            }
            Err(e) => eprintln!("  [{}/{total}] {}: request error: {e}", i + 1, pet.name),
        }
    }

    eprintln!("Filled evolution requirements for {found}/{total} pets.");
    Ok(())
}
