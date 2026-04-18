//! Tiny `{{capture}}` interpolation helper for query rules.

use std::collections::BTreeMap;

pub type CaptureMap = BTreeMap<String, String>;

pub fn interpolate(template: &str, captures: &CaptureMap) -> String {
    let mut out = String::with_capacity(template.len());
    let mut cursor = 0;

    while let Some(open_rel) = template[cursor..].find("{{") {
        let open = cursor + open_rel;
        out.push_str(&template[cursor..open]);

        let after_open = open + 2;
        let Some(close_rel) = template[after_open..].find("}}") else {
            out.push_str(&template[open..]);
            return out;
        };
        let close = after_open + close_rel;
        let name = &template[after_open..close];

        if name.is_empty() {
            out.push_str("{{}}");
        } else {
            let value = captures.get(name).unwrap_or_else(|| {
                panic!("bug: unknown capture `{name}` reached template interpolation")
            });
            out.push_str(value);
        }

        cursor = close + 2;
    }

    out.push_str(&template[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::{interpolate, CaptureMap};

    #[test]
    fn interpolates_named_captures() {
        let captures = CaptureMap::from([
            ("recv".to_string(), "client".to_string()),
            ("method".to_string(), "unwrap".to_string()),
        ]);

        assert_eq!(
            interpolate("avoid {{method}} on {{recv}}", &captures),
            "avoid unwrap on client"
        );
    }

    #[test]
    fn preserves_literal_unclosed_opening() {
        let captures = CaptureMap::new();
        assert_eq!(interpolate("literal {{ text", &captures), "literal {{ text");
    }
}
