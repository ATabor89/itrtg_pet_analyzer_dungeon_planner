# Save-field tracking targets (not yet located)

Currencies / consumables we want to find in the save. Method is the usual
before/after diff (buy or use some, re-export, diff scalar leaves — see
`Kongregate/experiments.md`). Most "currency" counts are likely dedicated
scalars in the `X` (pet) block or root; consumables may be material-inventory
(`X.Q`) entries.

| Target | What it is | Notes / where to look | Status |
|--------|-----------|------------------------|--------|
| **Pet Tokens** | Unlock / evolve pets; bought with pet stones | Player buys them in bulk → easy diff (buy or use some). Likely a dedicated `X.*` scalar. | TODO |
| **Class Change Tokens** (CC Token) | Re-class an evolved pet for free | Diff a buy/use. Dedicated scalar. | TODO |
| **Lucky Draws** | Gacha-style draw | `x.071` already = lucky draws **opened** (lifetime); the **unopened** draw count is a separate, unlocated field. | Partial |
| **Ultimate Shadow Summon** | Summon currency/item | Diff a gain/use. | TODO |
| **Godly Liquid** | Consumable; ×2 creating speed while active (Main Stats: "bonus from godly liquids (×2)") | Count likely a scalar or `X.Q` entry; an *active* timer may be separate. | TODO |
| **Godly Liquid V2** | Stronger variant | As above. | TODO |
| **Chakra Pill** | Consumable; ×2 building speed (Main Stats: "bonus from chakra pills (×2)") | As above. | TODO |
| **Chakra Pill V2** | Stronger variant | As above. | TODO |

Easiest to diff on the **real save** (just gain/spend a little): Lucky Draws,
Ultimate Shadow Summon, Godly Liquid / V2, Chakra Pill / V2. Pet Tokens are easy
on either (buy with pet stones).

## Future

- **Individual challenges** — track per-challenge state (completions, the spent
  `Chp X boost` levels) once past Baal. Candidate blocks: `root.015` (list of
  ~118 {id, …}) and the `root.y/z/A/B` achievement-style lists.
- **Baal / TBS progression** — the player is pushing to beat Baal to move more
  of these counters; revisit `root.T` (Baal Slayer), `root.S` (TBS levels),
  `root.P` (current god) deltas afterward.
