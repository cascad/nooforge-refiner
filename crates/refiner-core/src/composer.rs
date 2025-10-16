use crate::types::{Composite, Unit};


pub fn fuse_title_rollup_topics(title: &str, summary: &str, topics: &[String]) -> Composite {
Composite {
title: title.to_string(),
summary: summary.to_string(),
topics: topics.iter().map(|t| super::types::Topic{ text: t.clone(), lang: "auto".into() }).collect(),
}
}


pub fn select_head_units(units: &[Unit], max_chars: usize) -> Vec<Unit> {
let mut total = 0usize;
let mut out = Vec::new();
for u in units {
if total + u.text.len() > max_chars { break; }
total += u.text.len();
out.push(u.clone());
}
out
}