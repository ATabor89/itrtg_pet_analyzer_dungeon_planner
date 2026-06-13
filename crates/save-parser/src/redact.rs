//! Redaction of personally-identifying fields from a save's lossless tree.
//!
//! The repo is public and the committed reference saves embed real PII — the
//! player's Steam id, account names, and a god name that is the user's email
//! handle. All of it sits at the *root* of the tree (verified against every
//! reference save; nothing is mirrored inside a nested base64 block), so
//! redaction is a handful of in-place [`Raw::set_scalar`] calls that leave
//! every other byte untouched.
//!
//! Server timestamps (`005`) and the init-log string (`006`) are deliberately
//! left alone: they are not PII, and blanking them could change how the save
//! parses or loads.

use crate::raw::Raw;

/// Root struct keys that hold identity / account data, paired with the
/// placeholder each is replaced with. The Steam id placeholder keeps the
/// 17-digit numeric shape; the rest are obvious sentinels.
pub const IDENTITY_FIELDS: &[(&str, &str)] = &[
    ("s", "RedactedGod"),             // god name (was the player's email handle)
    ("W", "RedactedPlayer"),          // player name
    ("001", "00000000000000000"),     // Steam id64
    ("002", "RedactedAccount"),       // account name
    ("003", "a_0000000000000000000"), // account / guest id
    ("004", "Redacted Name"),         // display name
];

/// One field that redaction changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Redaction {
    pub key: String,
    pub old: String,
    pub new: String,
}

/// Replace every [`IDENTITY_FIELDS`] entry present at the root of `root`,
/// returning the changes actually made (fields that are absent or empty are
/// skipped). `root` must be the top-level struct from [`crate::raw::parse`].
pub fn redact_identity(root: &mut Raw) -> Vec<Redaction> {
    let mut changed = Vec::new();
    for (key, placeholder) in IDENTITY_FIELDS {
        if let Some(old) = root.set_scalar(key, placeholder)
            && old != *placeholder
        {
            changed.push(Redaction {
                key: (*key).to_string(),
                old,
                new: (*placeholder).to_string(),
            });
        }
    }
    changed
}

/// Post-redaction safety net: return any `needles` still present in `text`.
///
/// Pass the redacted-away values (the `old` of each [`Redaction`]) — if any
/// reappears, an identity value is mirrored somewhere redaction did not reach,
/// and the caller should refuse to write the file.
pub fn residual_hits<'a>(text: &str, needles: &[&'a str]) -> Vec<&'a str> {
    needles
        .iter()
        .copied()
        .filter(|n| !n.is_empty() && text.contains(n))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw;

    #[test]
    fn redacts_known_root_identity_fields() {
        let plaintext = "s:Shoggoth269;W:ShoggothUnknown;c:1781053129;\
                         001:76561198034867786;002:ShoggothUnknown;\
                         003:a_3391713700228783307;004:Shoggoth Unknown;005:123;";
        let mut root = raw::parse(plaintext);
        let changes = redact_identity(&mut root);

        // All six identity fields changed; the timestamp `c`/`005` did not.
        assert_eq!(changes.len(), 6);
        assert!(changes.iter().any(|c| c.key == "001" && c.old == "76561198034867786"));

        let out = root.serialize();
        // Every redacted value is gone, including the partial "Shoggoth".
        let olds: Vec<&str> = changes.iter().map(|c| c.old.as_str()).collect();
        assert!(residual_hits(&out, &olds).is_empty());
        assert!(!out.contains("Shoggoth"));
        assert!(!out.contains("76561198034867786"));

        // Untouched fields survive verbatim, in place.
        assert_eq!(root.get("c").unwrap(), &Raw::Scalar("1781053129".into()));
        assert_eq!(root.get("005").unwrap(), &Raw::Scalar("123".into()));
        assert_eq!(root.get("s").unwrap(), &Raw::Scalar("RedactedGod".into()));
    }

    #[test]
    fn redaction_is_idempotent_and_skips_absent_fields() {
        // Only `s` present; the rest are absent and must be skipped silently.
        let mut root = raw::parse("s:Shoggoth269;c:1;");
        assert_eq!(redact_identity(&mut root).len(), 1);
        // Running again finds nothing new to change.
        assert!(redact_identity(&mut root).is_empty());
    }

    #[test]
    fn residual_hits_flags_a_mirrored_value() {
        assert_eq!(residual_hits("a:1;b:secret;", &["secret"]), vec!["secret"]);
        assert!(residual_hits("a:1;b:clean;", &["secret"]).is_empty());
    }
}
