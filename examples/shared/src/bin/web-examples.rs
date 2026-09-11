//! Prints the website's example registry as JSON, for the Astro build.
//!
//!   cargo run -p rataflow-examples --bin web-examples
//!
//! The site needs each example's title, description and key list to build its
//! per-example routes. Those already exist, once, in `rataflow_examples::meta`
//! — so the site reads them from there rather than keeping a copy, which is the
//! same reason `meta` exists at all: the prose used to live in two places and
//! eight of twenty-one descriptions had drifted apart before it did.
//!
//! A Rust bin rather than a script that parses `meta.rs`: the registry is Rust
//! values, and anything that reads them by regex is a second parser to keep
//! correct. This one cannot disagree with what the app renders because it calls
//! the same functions.
//!
//! The module doc and the source of each example are NOT emitted here. They are
//! whole files, and the Astro page reads them straight off disk by the `source`
//! path in each record — no reason to push a few hundred lines of Rust through
//! a JSON string.

use rataflow_examples::meta;

/// Minimal JSON string escaping: the six characters the grammar requires, plus
/// the C0 range as \u00XX. Pulling serde in for three fields would mean turning
/// on this crate's `serde` feature (and `rataflow/serde` with it) just to run a
/// build script, which is a heavier dependency than the escaping it saves.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn main() {
    let examples = meta::web_examples();
    let mut out = String::from("[\n");
    for (i, ex) in examples.iter().enumerate() {
        out.push_str("  {\n");
        out.push_str(&format!("    \"slug\": \"{}\",\n", esc(ex.slug)));
        out.push_str(&format!("    \"source\": \"{}\",\n", esc(ex.source)));
        match ex.web_source {
            Some(w) => out.push_str(&format!("    \"webSource\": \"{}\",\n", esc(w))),
            None => out.push_str("    \"webSource\": null,\n"),
        }
        out.push_str(&format!("    \"title\": \"{}\",\n", esc(ex.meta.title)));
        match ex.meta.description {
            Some(d) => out.push_str(&format!("    \"description\": \"{}\",\n", esc(d))),
            // `events` has none: it draws its own event-log panel instead of the
            // shell's sidebar. The page falls back to the module doc's first
            // line, which is why that is worth having.
            None => out.push_str("    \"description\": null,\n"),
        }
        out.push_str("    \"keys\": [");
        for (j, (key, action)) in ex.meta.keys.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("[\"{}\", \"{}\"]", esc(key), esc(action)));
        }
        out.push_str("]\n");
        out.push_str(if i + 1 == examples.len() { "  }\n" } else { "  },\n" });
    }
    out.push_str("]\n");
    print!("{out}");
}
