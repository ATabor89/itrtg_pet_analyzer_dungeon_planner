//! Audit `data/wiki_pets.yaml` after a wiki refresh and report anything the
//! extractor had to guess at (placeholders like `?`, unrated difficulties,
//! uncurated campaign bonuses). Always exits 0 on findings: the refresh should
//! stay green, and the report is the alert.
//!
//! ```text
//! wiki-audit [--wiki data/wiki_pets.yaml] [--rules data/campaign_bonuses.yaml]
//!            [--parse-warnings <extractor warnings yaml>] [--markdown <out.md>]
//!            [--issue-markdown <out.md>]
//! ```
//!
//! `--markdown` gets every finding (for the job summary); `--issue-markdown`
//! leaves out one-off page-fetch problems, so the standing issue only changes
//! when the data does.
//!
//! Under GitHub Actions (`GITHUB_ACTIONS=true`) it also emits one `::warning`
//! annotation per finding, appends the report to `$GITHUB_STEP_SUMMARY`, and
//! writes `findings=<n>` (the issue-worthy count) to `$GITHUB_OUTPUT`.

use std::io::Write;

use itrtg_models::{CampaignBonusRules, WikiDataWarning, WikiPet};
use itrtg_planner::wiki_audit::{audit, is_persistent, render_markdown};

fn main() {
    if let Err(e) = run() {
        eprintln!("wiki-audit: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut wiki_path = "data/wiki_pets.yaml".to_string();
    let mut rules_path = "data/campaign_bonuses.yaml".to_string();
    let mut parse_warnings_path = None;
    let mut markdown_path = None;
    let mut issue_markdown_path = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--wiki" => wiki_path = value()?,
            "--rules" => rules_path = value()?,
            "--parse-warnings" => parse_warnings_path = Some(value()?),
            "--markdown" => markdown_path = Some(value()?),
            "--issue-markdown" => issue_markdown_path = Some(value()?),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }

    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    let wiki: Vec<WikiPet> =
        serde_yaml::from_str(&read(&wiki_path)?).map_err(|e| format!("{wiki_path}: {e}"))?;
    let rules: CampaignBonusRules =
        serde_yaml::from_str(&read(&rules_path)?).map_err(|e| format!("{rules_path}: {e}"))?;
    let parse_warnings: Vec<WikiDataWarning> = match &parse_warnings_path {
        // A missing file just means the extractor didn't get that far.
        Some(p) if std::path::Path::new(p).exists() => {
            serde_yaml::from_str::<Option<Vec<WikiDataWarning>>>(&read(p)?)
                .map_err(|e| format!("{p}: {e}"))?
                .unwrap_or_default()
        }
        _ => Vec::new(),
    };

    let findings = audit(&wiki, &rules, &parse_warnings);
    let md = render_markdown(&findings);
    print!("{md}");

    if let Some(p) = &markdown_path {
        std::fs::write(p, &md).map_err(|e| format!("{p}: {e}"))?;
    }
    let persistent: Vec<WikiDataWarning> = findings.iter().filter(|w| is_persistent(w)).cloned().collect();
    if let Some(p) = &issue_markdown_path {
        std::fs::write(p, render_markdown(&persistent)).map_err(|e| format!("{p}: {e}"))?;
    }

    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        for w in &findings {
            println!(
                "::warning title={}::{}: {}",
                escape_property(&format!("Wiki data: {}", w.pet)),
                escape_data(&w.field),
                escape_data(&w.detail)
            );
        }
        append_env_file("GITHUB_STEP_SUMMARY", &format!("## Wiki data audit\n\n{md}\n"))?;
        append_env_file("GITHUB_OUTPUT", &format!("findings={}\n", persistent.len()))?;
    }
    Ok(())
}

fn append_env_file(var: &str, text: &str) -> Result<(), String> {
    let Ok(path) = std::env::var(var) else { return Ok(()) };
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|e| format!("{path}: {e}"))?;
    f.write_all(text.as_bytes()).map_err(|e| format!("{path}: {e}"))
}

/// Workflow-command escaping for the message part.
fn escape_data(s: &str) -> String {
    s.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A")
}

/// Workflow-command escaping for a property value (also `:` and `,`).
fn escape_property(s: &str) -> String {
    escape_data(s).replace(':', "%3A").replace(',', "%2C")
}
