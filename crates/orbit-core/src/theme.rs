//! Theme mapping and contrast.
//!
//! Monkeytype palettes (see `themes/NOTICE`) are mapped onto Orbit's semantic
//! tokens with one of three strategies. Pure functions only: no clock, no I/O.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One Monkeytype palette, as shipped in `themes/monkeytype.json`.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub bg: String,
    pub caret: String,
    pub main: String,
    pub sub: String,
    pub sub_alt: String,
    pub text: String,
    pub error: String,
    pub error_extra: String,
    pub colorful_error: String,
    pub colorful_error_extra: String,
}

/// How a palette is mapped onto Orbit tokens.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    /// Orbit's own base theme with only the accent taken from the palette.
    Accent,
    /// Palette neutrals, with surfaces and muted text derived and corrected for contrast.
    Derived,
    /// Palette colours used as-is.
    Direct,
}

/// WCAG contrast grade of a palette's text.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Grade {
    Aaa,
    Aa,
    Low,
}

/// Orbit's light base, copied from `app/src/app.css` `:root`.
const ORBIT_LIGHT: &[(&str, &str)] = &[
    ("bg", "#fbfaf8"),
    ("side", "#f5f3ef"),
    ("surface", "#ffffff"),
    ("sunk", "#f2efea"),
    ("line", "#ebe7e0"),
    ("line-strong", "#dcd5ca"),
    ("text", "#2a2723"),
    ("muted", "#6b665e"),
    ("icon", "#8a847a"),
    ("accent", "#4f7f5b"),
    ("accent-ink", "#ffffff"),
    ("accent-soft", "#e4efe5"),
    ("accent-line", "#b9d3bd"),
    ("accent-text", "#3f6b4a"),
    ("danger", "#bb3a2a"),
    ("warn", "#9a5d08"),
];

/// Orbit's dark base, copied from `app/src/app.css` `[data-theme='dark']`.
const ORBIT_DARK: &[(&str, &str)] = &[
    ("bg", "#1b1a18"),
    ("side", "#211f1d"),
    ("surface", "#252321"),
    ("sunk", "#1e1d1b"),
    ("line", "#33302c"),
    ("line-strong", "#45413b"),
    ("text", "#ede8e0"),
    ("muted", "#a8a197"),
    ("icon", "#8f897f"),
    ("accent", "#93c39b"),
    ("accent-ink", "#1b1a18"),
    ("accent-soft", "#1f2d22"),
    ("accent-line", "#3e5b45"),
    ("accent-text", "#a8d2af"),
    ("danger", "#ef8070"),
    ("warn", "#e2ab5a"),
];

/// Every bundled palette, keyed by Monkeytype's theme name.
pub fn palettes() -> BTreeMap<String, Palette> {
    serde_json::from_str(include_str!("themes/monkeytype.json"))
        .expect("bundled palettes are valid JSON")
}

/// Parses `#rgb` or `#rrggbb`.
///
/// # Panics
/// Panics on anything else; callers pass bundled palette data.
fn rgb(hex: &str) -> [u8; 3] {
    fn invalid(hex: &str) -> ! {
        panic!("not a hex colour: {hex}")
    }
    let Some(h) = hex.strip_prefix('#') else {
        invalid(hex)
    };
    if !h.is_ascii() || !(h.len() == 3 || h.len() == 6) {
        invalid(hex);
    }
    let full: String = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect()
    } else {
        h.to_string()
    };
    let channel =
        |i: usize| u8::from_str_radix(&full[i..i + 2], 16).unwrap_or_else(|_| invalid(hex));
    [channel(0), channel(2), channel(4)]
}

fn luminance(hex: &str) -> f64 {
    let [r, g, b] = rgb(hex).map(|v| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG contrast ratio between two `#rgb` or `#rrggbb` colours.
pub fn contrast(a: &str, b: &str) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// sRGB mix: `pct_a` percent of `a` and the rest `b`. Returns `#rrggbb`.
fn mix(a: &str, b: &str, pct_a: f64) -> String {
    let (x, y) = (rgb(a), rgb(b));
    let t = (pct_a / 100.0).clamp(0.0, 1.0);
    let ch = |i: usize| (f64::from(x[i]) * t + f64::from(y[i]) * (1.0 - t)).round() as u8;
    format!("#{:02x}{:02x}{:02x}", ch(0), ch(1), ch(2))
}

/// Moves `from` toward `toward` in 5% steps and returns the first colour with
/// at least `min` contrast on `bg`. Falls back to `toward` itself.
fn toward_until(from: &str, toward: &str, bg: &str, min: f64) -> String {
    (0..=20_i32)
        .map(|step| mix(toward, from, f64::from(step * 5)))
        .find(|c| contrast(c, bg) >= min)
        .unwrap_or_else(|| toward.to_string())
}

/// Contrast grade: Aaa for text >= 7:1 and sub >= 4.5:1, Aa for text >= 4.5:1
/// and sub >= 3:1, otherwise Low.
pub fn grade(p: &Palette) -> Grade {
    let t = contrast(&p.text, &p.bg);
    let s = contrast(&p.sub, &p.bg);
    if t >= 7.0 && s >= 4.5 {
        Grade::Aaa
    } else if t >= 4.5 && s >= 3.0 {
        Grade::Aa
    } else {
        Grade::Low
    }
}

fn score(p: &Palette) -> f64 {
    contrast(&p.text, &p.bg).min(contrast(&p.sub, &p.bg) * 1.5)
}

/// Strategies that may be offered for a palette, in display order.
pub fn offered(p: &Palette) -> Vec<Strategy> {
    let text = contrast(&p.text, &p.bg);
    let mut out = Vec::new();
    if text >= 7.0 && contrast(&p.sub, &p.bg) >= 4.5 {
        out.push(Strategy::Direct);
    }
    if text >= 4.5 {
        out.push(Strategy::Derived);
    }
    out.push(Strategy::Accent);
    out
}

/// The four highest-scoring palettes with grade Aaa, best first.
pub fn featured() -> Vec<String> {
    let mut picks: Vec<(String, f64)> = palettes()
        .into_iter()
        .filter(|(_, p)| grade(p) == Grade::Aaa)
        .map(|(name, p)| {
            let s = score(&p);
            (name, s)
        })
        .collect();
    picks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    picks.into_iter().take(4).map(|(name, _)| name).collect()
}

fn base(entries: &[(&'static str, &str)]) -> BTreeMap<&'static str, String> {
    entries.iter().map(|&(k, v)| (k, v.to_string())).collect()
}

fn direct(p: &Palette) -> BTreeMap<&'static str, String> {
    [
        ("bg", p.bg.clone()),
        ("side", p.sub_alt.clone()),
        ("surface", p.bg.clone()),
        ("sunk", p.sub_alt.clone()),
        ("line", p.sub_alt.clone()),
        ("line-strong", p.sub.clone()),
        ("text", p.text.clone()),
        ("muted", p.sub.clone()),
        ("icon", p.sub.clone()),
        ("accent", p.main.clone()),
        ("accent-ink", p.bg.clone()),
        ("accent-soft", mix(&p.main, &p.bg, 16.0)),
        ("accent-line", mix(&p.main, &p.bg, 45.0)),
        ("accent-text", p.main.clone()),
        ("danger", p.error.clone()),
        ("warn", p.colorful_error.clone()),
    ]
    .into_iter()
    .collect()
}

fn derived(p: &Palette) -> BTreeMap<&'static str, String> {
    let mut t = direct(p);
    t.insert("surface", mix(&p.text, &p.bg, 4.0));
    t.insert("side", mix(&p.text, &p.bg, 3.0));
    t.insert("sunk", mix(&p.text, &p.bg, 7.0));
    t.insert("line", mix(&p.text, &p.bg, 14.0));
    t.insert("line-strong", mix(&p.text, &p.bg, 24.0));
    let muted = toward_until(&p.sub, &p.text, &p.bg, 4.5);
    t.insert("muted", muted.clone());
    t.insert("icon", muted);
    t.insert("accent-text", toward_until(&p.main, &p.text, &p.bg, 4.5));
    t
}

fn accent(p: &Palette) -> BTreeMap<&'static str, String> {
    let mut t = if luminance(&p.bg) > 0.18 {
        base(ORBIT_LIGHT)
    } else {
        base(ORBIT_DARK)
    };
    let bg = t["bg"].clone();
    let text = t["text"].clone();
    t.insert("accent", p.main.clone());
    t.insert("accent-soft", mix(&p.main, &bg, 14.0));
    t.insert("accent-line", mix(&p.main, &bg, 45.0));
    t.insert("accent-text", toward_until(&p.main, &text, &bg, 4.5));
    let ink = if contrast(&bg, &p.main) >= contrast(&text, &p.main) {
        bg
    } else {
        text
    };
    t.insert("accent-ink", ink);
    t
}

/// Maps a palette onto Orbit's semantic tokens using `s`.
pub fn tokens(p: &Palette, s: Strategy) -> BTreeMap<&'static str, String> {
    match s {
        Strategy::Direct => direct(p),
        Strategy::Derived => derived(p),
        Strategy::Accent => accent(p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_contrast_is_twenty_one() {
        assert!((contrast("#000", "#fff") - 21.0).abs() < 0.01);
    }

    #[test]
    fn every_offered_theme_meets_its_contrast_grade() {
        for (name, p) in palettes() {
            for s in offered(&p) {
                let t = tokens(&p, s);
                let bg = &t["bg"];
                for key in ["text", "muted", "accent-text"] {
                    let c = contrast(&t[key], bg);
                    assert!(c >= 4.5, "{name} {s:?} {key}: {c:.2} < 4.5");
                }
                if s == Strategy::Direct {
                    let c = contrast(&t["text"], bg);
                    assert!(c >= 7.0, "{name} direct text: {c:.2} < 7");
                }
            }
        }
    }

    #[test]
    fn featured_has_up_to_four_aaa_palettes_sorted_by_score() {
        let names = featured();
        assert!((1..=4).contains(&names.len()), "got {}", names.len());
        let all = palettes();
        for n in &names {
            assert_eq!(grade(&all[n]), Grade::Aaa, "{n}");
        }
        let scores: Vec<f64> = names.iter().map(|n| score(&all[n])).collect();
        assert!(scores.windows(2).all(|w| w[0] >= w[1]), "{scores:?}");
    }

    #[test]
    fn bundled_palettes_include_at_least_twenty_themes() {
        assert!(palettes().len() >= 20);
    }
}
