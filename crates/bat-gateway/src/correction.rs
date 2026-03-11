//! User correction and preference detection.
//!
//! Scans user messages for patterns that indicate corrections, preferences,
//! or instructions to remember something.

use bat_types::memory::ObservationKind;

/// A detected correction or preference in a user message.
pub struct Detection {
    pub kind: ObservationKind,
    pub key: String,
    pub value: String,
}

/// Scan a user message for correction/preference patterns.
/// Returns all detections found.
pub fn detect(message: &str) -> Vec<Detection> {
    let lower = message.to_lowercase();
    let mut results = Vec::new();

    // Correction patterns: "don't do X", "stop doing X", "never X"
    let correction_patterns: &[(&str, fn(&str, &str) -> Option<String>)] = &[
        ("don't do ", |_, l| extract_after(l, "don't do ")),
        ("dont do ", |_, l| extract_after(l, "dont do ")),
        ("do not do ", |_, l| extract_after(l, "do not do ")),
        ("stop doing ", |_, l| extract_after(l, "stop doing ")),
        ("never ", |_, l| extract_after(l, "never ")),
        ("don't ", |_, l| extract_after(l, "don't ")),
        ("do not ", |_, l| extract_after(l, "do not ")),
    ];

    for (pattern, extractor) in correction_patterns {
        if lower.contains(pattern) {
            if let Some(value) = extractor(message, &lower) {
                results.push(Detection {
                    kind: ObservationKind::UserCorrection,
                    key: "user_correction".to_string(),
                    value,
                });
                break; // One correction per message is enough
            }
        }
    }

    // Remember patterns: "remember that X", "keep in mind X"
    let remember_patterns: &[&str] = &[
        "remember that ",
        "remember this",
        "keep in mind ",
        "keep in mind,",
    ];
    for pattern in remember_patterns {
        if lower.contains(pattern) {
            let value = extract_after(&lower, pattern)
                .unwrap_or_else(|| message.to_string());
            results.push(Detection {
                kind: ObservationKind::Preference,
                key: "user_reminder".to_string(),
                value,
            });
            break;
        }
    }

    // Preference patterns: "I prefer X", "I like X", "always do X", "use X instead"
    let preference_patterns: &[(&str, &str)] = &[
        ("i prefer ", "preference"),
        ("i like ", "preference"),
        ("always do ", "preference"),
        ("always use ", "preference"),
        ("use ", "preference"),      // "use X instead"
        ("instead do ", "preference"),
        ("instead, do ", "preference"),
        ("rather than ", "preference"),
    ];

    for (pattern, key) in preference_patterns {
        if lower.contains(pattern) {
            // For "use X instead", grab the whole relevant sentence
            let value = extract_after(&lower, pattern)
                .unwrap_or_else(|| message.to_string());
            // Only count if the extracted value is meaningful (>3 chars)
            if value.len() > 3 {
                results.push(Detection {
                    kind: ObservationKind::Preference,
                    key: key.to_string(),
                    value,
                });
                break;
            }
        }
    }

    results
}

/// Extract text after a pattern until end of sentence or message.
fn extract_after(text: &str, pattern: &str) -> Option<String> {
    let idx = text.find(pattern)?;
    let rest = &text[idx + pattern.len()..];
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Take until end of sentence
    let end = trimmed
        .find(|c: char| c == '.' || c == '!' || c == '?' || c == '\n')
        .unwrap_or(trimmed.len());
    let result = trimmed[..end].trim().to_string();
    if result.is_empty() { None } else { Some(result) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_correction() {
        let dets = detect("Don't do that again please");
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].kind, ObservationKind::UserCorrection);
    }

    #[test]
    fn detect_never() {
        let dets = detect("Never use tabs for indentation");
        assert!(dets.iter().any(|d| d.kind == ObservationKind::UserCorrection));
        assert!(dets[0].value.contains("use tabs"));
    }

    #[test]
    fn detect_preference() {
        let dets = detect("I prefer spaces over tabs");
        assert!(dets.iter().any(|d| d.kind == ObservationKind::Preference));
    }

    #[test]
    fn detect_remember() {
        let dets = detect("Remember that I use vim keybindings");
        assert!(dets.iter().any(|d| d.kind == ObservationKind::Preference && d.value.contains("vim")));
    }

    #[test]
    fn detect_nothing() {
        let dets = detect("How's the weather today?");
        assert!(dets.is_empty());
    }

    #[test]
    fn detect_always() {
        let dets = detect("Always use TypeScript instead of JavaScript");
        assert!(dets.iter().any(|d| d.kind == ObservationKind::Preference));
    }
}
