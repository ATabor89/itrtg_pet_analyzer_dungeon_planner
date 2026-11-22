use clap::Parser;
use itrtg_models::{WikiDataWarning, WikiPet};
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

    /// Write data warnings (placeholder/unrecognized values, unreadable pages)
    /// to this YAML file. The run still succeeds; the refresh workflow turns
    /// the file into an alert.
    #[arg(long)]
    warnings: Option<PathBuf>,

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

    let (mut pets, mut warnings) = parser::parse_pets_with_warnings(&source)?;

    if !cli.skip_evo {
        crawl_evo_requirements(&mut pets, cli.evo_delay_ms, &mut warnings)?;
    }

    for w in &warnings {
        eprintln!("Warning: {}: {}: {}", w.pet, w.field, w.detail);
    }
    if let Some(path) = &cli.warnings {
        std::fs::write(path, serde_yaml::to_string(&warnings)?)?;
        eprintln!("Wrote {} data warnings to {}", warnings.len(), path.display());
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

/// Visit each pet's rendered wiki page and fill in the per-pet infobox data
/// (evolution requirements and campaign bonus) from a single fetch.
///
/// This is the slow path (one request per pet), so it only runs in the
/// extractor/CI — never in the app, which reads the baked-in YAML. Failures for
/// individual pets are logged and skipped: a missing page or block just leaves
/// the field as `None` rather than aborting the whole run.
fn crawl_evo_requirements(
    pets: &mut [WikiPet],
    delay_ms: u64,
    warnings: &mut Vec<WikiDataWarning>,
) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("pet_extractor/0.1.0 (ITRTG tool)")
        .build()?;

    let total = pets.len();
    let (mut evo_found, mut camp_found) = (0usize, 0usize);
    eprintln!("Crawling per-pet pages (evolution + campaign bonus) for {total} pets...");

    for (i, pet) in pets.iter_mut().enumerate() {
        // Be polite: space out requests so we don't hammer the wiki.
        if i > 0 && delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }

        let mut warn = |detail: String| {
            warnings.push(WikiDataWarning {
                pet: pet.name.clone(),
                // Keep in sync with `wiki_audit::WIKI_PAGE_FIELD` (planner),
                // which keeps these one-off fetch problems out of the issue.
                field: "wiki page".to_string(),
                detail,
            })
        };
        match client.get(&pet.wiki_url).send() {
            Ok(resp) if resp.status().is_success() => match resp.text() {
                Ok(html) => {
                    if let Some(evo) = parser::parse_evo_requirements(&html) {
                        pet.evo_requirements = Some(evo);
                        evo_found += 1;
                    } else if let Some(cell) = parser::evo_growth_cell(&html) {
                        // The block is there but the threshold isn't a number
                        // (typically `?` on a brand-new pet).
                        warnings.push(WikiDataWarning {
                            pet: pet.name.clone(),
                            field: "evo requirements".to_string(),
                            detail: format!("growth threshold {:?} is not a number; left unset", cell),
                        });
                    } else {
                        eprintln!("  [{}/{total}] {}: no evolution block found", i + 1, pet.name);
                    }
                    if let Some(cb) = parser::parse_campaign_bonus(&html) {
                        pet.campaign_bonus = Some(cb);
                        camp_found += 1;
                    }
                }
                Err(e) => warn(format!("read error: {e}")),
            },
            Ok(resp) => warn(format!("HTTP {}", resp.status())),
            Err(e) => warn(format!("request error: {e}")),
        }
    }

    eprintln!("Filled evolution requirements for {evo_found}/{total}, campaign bonuses for {camp_found}/{total} pets.");
    Ok(())
}
