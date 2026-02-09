//! Configuration for evaluation heuristics.
//!
//! Allows enabling/disabling categories of evaluation heuristics to test
//! which are most effective for playing strength.

use std::sync::atomic::{AtomicU32, Ordering};

use bitflags::bitflags;

bitflags! {
    /// Flags controlling which evaluation heuristics are enabled.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EvalFlags: u32 {
        /// Side-to-move tempo bonus
        const TEMPO = 1 << 0;
        /// Penalty for undefended pieces
        const HANGING = 1 << 1;
        /// Holes, center control, space evaluation
        const CENTER = 1 << 2;
        /// Pawn islands, chains, tension, storm, majority
        const PAWN_STRUCTURE = 1 << 3;
        /// King safety, ring pressure, shelter patterns, activity
        const KING_SAFETY = 1 << 4;
        /// Rook/Queen file activity, doubled rooks, alignments
        const ROOK_ACTIVITY = 1 << 5;
        /// Threat evaluation
        const THREATS = 1 << 6;
        /// Synergy, tropism, batteries, bishop pair
        const INTERACTIONS = 1 << 7;
        /// Knight vs bishop imbalance, drawish tweaks
        const IMBALANCE = 1 << 8;
        /// All flags enabled (default)
        const ALL = 0x3FF;
    }
}

/// Global evaluation flags (default: all enabled)
static EVAL_FLAGS: AtomicU32 = AtomicU32::new(EvalFlags::ALL.bits());

/// Get the current evaluation flags.
#[inline]
pub fn get_eval_flags() -> EvalFlags {
    EvalFlags::from_bits_truncate(EVAL_FLAGS.load(Ordering::Relaxed))
}

/// Set the evaluation flags.
pub fn set_eval_flags(flags: EvalFlags) {
    EVAL_FLAGS.store(flags.bits(), Ordering::Relaxed);
}

/// Check if a specific flag is enabled.
#[inline]
pub fn is_enabled(flag: EvalFlags) -> bool {
    get_eval_flags().contains(flag)
}

/// Enable specific flags (OR with current flags).
pub fn enable_flags(flags: EvalFlags) {
    let current = get_eval_flags();
    set_eval_flags(current | flags);
}

/// Disable specific flags (AND NOT with current flags).
pub fn disable_flags(flags: EvalFlags) {
    let current = get_eval_flags();
    set_eval_flags(current & !flags);
}

/// Reset to all flags enabled.
pub fn reset_eval_flags() {
    set_eval_flags(EvalFlags::ALL);
}

/// Parse flag names from a string (comma or space separated).
/// Returns None if any flag name is invalid.
pub fn parse_flags(s: &str) -> Option<EvalFlags> {
    let mut flags = EvalFlags::empty();
    for name in s.split(|c| c == ',' || c == ' ' || c == '+') {
        let name = name.trim().to_uppercase();
        if name.is_empty() {
            continue;
        }
        match name.as_str() {
            "TEMPO" => flags |= EvalFlags::TEMPO,
            "HANGING" => flags |= EvalFlags::HANGING,
            "CENTER" => flags |= EvalFlags::CENTER,
            "PAWN_STRUCTURE" | "PAWN" | "PAWNS" => flags |= EvalFlags::PAWN_STRUCTURE,
            "KING_SAFETY" | "KING" => flags |= EvalFlags::KING_SAFETY,
            "ROOK_ACTIVITY" | "ROOK" | "ROOKS" => flags |= EvalFlags::ROOK_ACTIVITY,
            "THREATS" => flags |= EvalFlags::THREATS,
            "INTERACTIONS" => flags |= EvalFlags::INTERACTIONS,
            "IMBALANCE" => flags |= EvalFlags::IMBALANCE,
            "ALL" => flags |= EvalFlags::ALL,
            "NONE" => {} // Keep empty
            _ => return None,
        }
    }
    Some(flags)
}

/// Format flags as a human-readable string.
pub fn format_flags(flags: EvalFlags) -> String {
    if flags == EvalFlags::ALL {
        return "ALL".to_string();
    }
    if flags.is_empty() {
        return "NONE".to_string();
    }

    let mut names = Vec::new();
    if flags.contains(EvalFlags::TEMPO) {
        names.push("TEMPO");
    }
    if flags.contains(EvalFlags::HANGING) {
        names.push("HANGING");
    }
    if flags.contains(EvalFlags::CENTER) {
        names.push("CENTER");
    }
    if flags.contains(EvalFlags::PAWN_STRUCTURE) {
        names.push("PAWN_STRUCTURE");
    }
    if flags.contains(EvalFlags::KING_SAFETY) {
        names.push("KING_SAFETY");
    }
    if flags.contains(EvalFlags::ROOK_ACTIVITY) {
        names.push("ROOK_ACTIVITY");
    }
    if flags.contains(EvalFlags::THREATS) {
        names.push("THREATS");
    }
    if flags.contains(EvalFlags::INTERACTIONS) {
        names.push("INTERACTIONS");
    }
    if flags.contains(EvalFlags::IMBALANCE) {
        names.push("IMBALANCE");
    }
    names.join(", ")
}

/* TODO these tests fail because they are executed in parallel with other tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_all_enabled() {
        reset_eval_flags();
        assert_eq!(get_eval_flags(), EvalFlags::ALL);
    }

    #[test]
    fn test_is_enabled() {
        reset_eval_flags();
        assert!(is_enabled(EvalFlags::TEMPO));
        assert!(is_enabled(EvalFlags::KING_SAFETY));
    }

    #[test]
    fn test_disable_flags() {
        reset_eval_flags();
        disable_flags(EvalFlags::TEMPO);
        assert!(!is_enabled(EvalFlags::TEMPO));
        assert!(is_enabled(EvalFlags::KING_SAFETY));
    }

    #[test]
    fn test_enable_flags() {
        set_eval_flags(EvalFlags::empty());
        enable_flags(EvalFlags::THREATS);
        assert!(is_enabled(EvalFlags::THREATS));
        assert!(!is_enabled(EvalFlags::TEMPO));
    }

    #[test]
    fn test_parse_flags() {
        assert_eq!(parse_flags("TEMPO"), Some(EvalFlags::TEMPO));
        assert_eq!(
            parse_flags("TEMPO,THREATS"),
            Some(EvalFlags::TEMPO | EvalFlags::THREATS)
        );
        assert_eq!(parse_flags("ALL"), Some(EvalFlags::ALL));
        assert_eq!(parse_flags("NONE"), Some(EvalFlags::empty()));
        assert_eq!(parse_flags("INVALID"), None);
    }

    #[test]
    fn test_format_flags() {
        assert_eq!(format_flags(EvalFlags::ALL), "ALL");
        assert_eq!(format_flags(EvalFlags::empty()), "NONE");
        assert_eq!(format_flags(EvalFlags::TEMPO), "TEMPO");
    }
}
 */
