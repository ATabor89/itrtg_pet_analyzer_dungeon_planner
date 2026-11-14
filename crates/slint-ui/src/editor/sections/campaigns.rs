//! Campaigns: the campaign slots (`X.x`, one per campaign type). Elapsed time
//! (`c`) counts up to the target (`e`); setting elapsed = target force-completes
//! an active campaign. The type is the slot's identity and pet assignments are
//! display-only here.
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{fmt_hours, owned, percent, scalar_noted, stage_button, Check, F};
use crate::form::{muted, spacer, text};

pub fn build(form: &mut F, session: &EditSession) {
    form.title("Campaigns");
    // Edits address slots by raw list index, which only holds when `X.x` is a
    // real list (a degenerate lone struct can't be indexed that way).
    if !matches!(session.root().get_path(&["X", "x"]), Some(Raw::List(_))) {
        form.note("Campaign data isn't in the expected multi-slot list form — edit via the Raw Save Tree.");
        return;
    }
    let Some(save) = session.derived() else {
        form.note("Typed campaign data unavailable for this save — use the Raw Save Tree.");
        return;
    };
    form.note("A campaign completes when elapsed ≥ target. Force-complete an active campaign (or set its elapsed directly) so rewards are ready on next load. The campaign type is the slot's identity, not editable here.");
    if save.campaigns.is_empty() {
        form.note("No campaign slots in this save.");
        return;
    }
    let name = |index: usize| {
        let campaign = &save.campaigns[index];
        campaign.campaign_type_name().map(str::to_string).unwrap_or_else(|| format!("Type {}", campaign.campaign_type_id))
    };
    let complete = |index: usize| (owned(&["X", "x", &index.to_string(), "c"]), format!("{} campaign elapsed", name(index)), save.campaigns[index].duration_ms.to_string());
    let all: Vec<_> = (0..save.campaigns.len()).filter(|&i| save.campaigns[i].duration_ms > 0).map(complete).collect();
    let done = format!("Completed {} campaign(s)", all.len());
    form.row(vec![stage_button(form, "campaigns/complete-all", "Complete all campaigns", all, done).w(190.0).hint("Force-complete every active campaign (set elapsed = target)"), spacer()]);
    form.header(vec![text("Campaign").w(130.0), text("Elapsed (ms)").w(130.0), text("Target").w(150.0), text("%").w(50.0), spacer().w(130.0), text("Pets")]);
    form.pin();
    for (index, campaign) in save.campaigns.iter().enumerate() {
        let title = text(name(index)).w(130.0).hint(format!("type id {} · bonus {}", campaign.campaign_type_id, campaign.bonus));
        // A slot with a non-zero target duration is running a campaign.
        if campaign.duration_ms == 0 {
            form.row(vec![title, muted("inactive").size(11.0).w(130.0), muted("—").w(150.0), spacer()]);
            continue;
        }
        let pets: Vec<String> = campaign.pet_type_ids.iter()
            .map(|id| save.pet_by_type_id(*id).map(|p| p.name.clone()).unwrap_or_else(|| format!("type {id}"))).collect();
        let pets = if pets.is_empty() { "—".to_string() } else { pets.join(", ") };
        let label = format!("{} campaign elapsed", name(index));
        form.row(vec![
            title,
            scalar_noted(form, session, &["X", "x", &index.to_string(), "c"], &label, Check::Finite, "Staged campaign elapsed").w(130.0),
            text(format!("{} ({})", campaign.duration_ms, fmt_hours(campaign.duration_ms as f64))).mono().size(11.0).w(150.0),
            text(format!("{:.0}%", percent(campaign.elapsed_ms, campaign.duration_ms))).w(50.0),
            stage_button(form, format!("campaigns/complete/{index}"), "Force complete", vec![complete(index)], "Staged campaign elapsed").w(130.0)
                .hint("Set elapsed = target so the campaign is ready to collect"),
            text(pets.clone()).size(11.0).hint(pets),
        ]);
    }
}
