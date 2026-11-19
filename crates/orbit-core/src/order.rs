//! Fractional indexing: string keys over base-62 digits that sort between any two neighbours.
const DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn digit(c: u8) -> usize {
    DIGITS.iter().position(|&d| d == c).unwrap_or(0)
}

/// A key strictly between `a` and `b` (`None` is an open end). If `a >= b` there is no such
/// key, so it returns one after `a`.
pub fn between(a: Option<&str>, b: Option<&str>) -> String {
    let a = a.unwrap_or("");
    let b = b.filter(|b| a < *b);
    mid(a.as_bytes(), b.map(str::as_bytes))
}

fn mid(a: &[u8], b: Option<&[u8]>) -> String {
    if let Some(b) = b {
        let mut n = 0;
        while n < b.len() && a.get(n).copied().unwrap_or(DIGITS[0]) == b[n] {
            n += 1;
        }
        if n == b.len() {
            return mid(a, None);
        }
        if n > 0 {
            let rest = mid(a.get(n..).unwrap_or(&[]), Some(&b[n..]));
            return format!("{}{rest}", String::from_utf8_lossy(&b[..n]));
        }
    }
    let da = a.first().map_or(0, |&c| digit(c));
    let db = b.map_or(DIGITS.len(), |b| digit(b[0]));
    if db - da > 1 {
        return (DIGITS[(da + db) / 2] as char).to_string();
    }
    match b {
        Some(b) if b.len() > 1 => (DIGITS[db] as char).to_string(),
        _ => format!(
            "{}{}",
            DIGITS[da] as char,
            mid(a.get(1..).unwrap_or(&[]), None)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn between_open_ends_gives_a_middle_key() {
        assert_eq!(between(None, None), "V");
        assert!(between(Some("V"), None).as_str() > "V");
        assert!(between(None, Some("V")).as_str() < "V");
    }

    #[test]
    fn between_table() {
        let cases = [
            (None, None),
            (Some("a"), Some("b")),
            (Some("a"), Some("a1")),
            (Some("1"), Some("2")),
            (Some("0V"), Some("1")),
            (Some("zz"), None),
            (None, Some("1")),
            (None, Some("01")),
            (Some("V"), Some("W")),
            (Some("Vz"), Some("W")),
            (Some("A"), Some("A1")),
        ];
        for (a, b) in cases {
            let m = between(a, b);
            assert!(a.is_none_or(|a| a < m.as_str()), "{a:?} {b:?} -> {m}");
            assert!(b.is_none_or(|b| m.as_str() < b), "{a:?} {b:?} -> {m}");
            assert!(!m.ends_with('0'), "{m}");
        }
    }

    #[test]
    fn a_thousand_random_inserts_stay_sorted_and_unique() {
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize
        };
        let mut keys: Vec<String> = Vec::new();
        for _ in 0..1000 {
            let at = next() % (keys.len() + 1);
            let a = at.checked_sub(1).map(|i| keys[i].as_str());
            let b = keys.get(at).map(String::as_str);
            let k = between(a, b);
            keys.insert(at, k);
        }
        assert!(keys.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn repeated_appends_and_prepends_stay_ordered() {
        let mut last = between(None, None);
        for _ in 0..200 {
            let next = between(Some(&last), None);
            assert!(next > last);
            last = next;
        }
        let mut first = between(None, None);
        for _ in 0..200 {
            let prev = between(None, Some(&first));
            assert!(prev < first);
            first = prev;
        }
    }
}
