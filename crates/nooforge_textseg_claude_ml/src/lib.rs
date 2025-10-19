
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Kind {
    Text, Code, List, Table, Quote, Terminal, Email, FrontMatter, NonContent, Formula, Art,
}

pub mod annotator;

impl Kind {
    pub fn from_marker_type(s: &str) -> Kind {
        match s {
            "paragraph" | "heading" | "mixed_script_text" | "html" => Kind::Text,
            "list" => Kind::List,
            "table" => Kind::Table,
            "code" => Kind::Code,
            "log" => Kind::Terminal,
            "email" => Kind::Email,
            "front_matter" => Kind::FrontMatter,
            "horizontal_rule" | "garbage" | "noise" => Kind::NonContent,
            "math" => Kind::Formula,
            _ => Kind::Text,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Text => "Text",
            Kind::Code => "Code",
            Kind::List => "List",
            Kind::Table => "Table",
            Kind::Quote => "Quote",
            Kind::Terminal => "Terminal",
            Kind::Email => "Email",
            Kind::FrontMatter => "FrontMatter",
            Kind::NonContent => "NonContent",
            Kind::Formula => "Formula",
            Kind::Art => "Art",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: i64,
    pub start: usize,
    pub end: usize, // exclusive
    pub kind: Kind,
    pub confidence: f32,
    #[serde(default)]
    pub title_hint: Option<String>,
    #[serde(default)]
    pub lang_hint: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub subtype: Option<String>,
}

#[derive(Error, Debug)]
pub enum SegError {
    #[error("no start marker at byte 0; found text before first marker at byte {0}")]
    LeadingText(usize),
    #[error("start marker without end marker for id {0}")]
    Unclosed(i64),
    #[error("mismatched end marker id {found} (expected {expected}) at byte {at}")]
    MismatchedEnd { expected: i64, found: i64, at: usize },
    #[error("byte-exact verification failed for block id={id}: expected slice [{expected_start}..{expected_end}) equals model slice len={model_len}")]
    ByteMismatch { id: i64, expected_start: usize, expected_end: usize, model_len: usize },
    #[error("trailing text after last end marker at byte {0}")]
    TrailingText(usize),
    #[error("blocks.json is missing metadata for id {0}")]
    MissingMeta(i64),
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockMeta {
    pub id: i64,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
}

fn take_start_id_at(text: &str, pos: usize, re_block: &Regex, re_short: &Regex) -> Option<(i64, usize)> {
    let sub = &text[pos..];

    if let Some(caps) = re_block.captures(sub) {
        let m = caps.get(0).unwrap();
        if m.start() == 0 {
            let id: i64 = caps.get(1).unwrap().as_str().parse().ok()?;
            return Some((id, m.end()));
        }
    }
    if let Some(caps) = re_short.captures(sub) {
        let m = caps.get(0).unwrap();
        if m.start() == 0 {
            let id: i64 = caps.get(1).unwrap().as_str().parse().ok()?;
            return Some((id, m.end()));
        }
    }
    None
}

/// Parse annotated text with minimal markers and verify byte-identical echo to original.
/// Start markers:  <<<BLOCK:N>>>  OR  <<<N>>>
/// End markers:    <<<END:N>>>
pub fn parse_with_markers(original_text: &str, annotated_text: &str, metas: &[BlockMeta]) -> Result<Vec<Segment>, SegError> {
    let bytes_orig = original_text.as_bytes();
    let b_annot = annotated_text.as_bytes();

    let re_start_block = Regex::new(r"(?s)<<<BLOCK:(\d+)>>>").unwrap();
    let re_start_short = Regex::new(r"(?s)<<<(\d+)>>>").unwrap();
    let re_end_block   = Regex::new(r"(?s)<<<END:(\d+)>>>").unwrap();

    if take_start_id_at(annotated_text, 0, &re_start_block, &re_start_short).is_none() {
        return Err(SegError::LeadingText(0));
    }

    let mut segs: Vec<Segment> = Vec::new();
    let mut cur_ann = 0usize;
    let mut cur_org = 0usize;

    while cur_ann < b_annot.len() {
        let (id, start_len) = match take_start_id_at(annotated_text, cur_ann, &re_start_block, &re_start_short) {
            Some(v) => v,
            None => break,
        };
        cur_ann += start_len;

        // find end <<<END:id>>> after cur_ann
        let mut end_abs_start: Option<usize> = None;
        let mut end_abs_end: Option<usize> = None;
        for caps in re_end_block.captures_iter(&annotated_text[cur_ann..]) {
            let m = caps.get(0).unwrap();
            let found_id: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(-1);
            if found_id == id {
                end_abs_start = Some(cur_ann + m.start());
                end_abs_end   = Some(cur_ann + m.end());
                break;
            }
        }
        let end_start = end_abs_start.ok_or_else(|| SegError::Unclosed(id))?;
        let end_end   = end_abs_end.unwrap();

        let body_len = end_start - cur_ann;
        let expected_end = cur_org + body_len;
        if expected_end > bytes_orig.len() ||
           &bytes_orig[cur_org..expected_end] != &b_annot[cur_ann..end_start] {
            return Err(SegError::ByteMismatch { id, expected_start: cur_org, expected_end, model_len: body_len });
        }

        let meta = metas.iter().find(|m| m.id == id).ok_or(SegError::MissingMeta(id))?;
        let kind = Kind::from_marker_type(&meta.type_);
        segs.push(Segment {
            id,
            start: cur_org,
            end: expected_end,
            kind,
            confidence: meta.confidence.unwrap_or(1.0),
            title_hint: meta.title.clone(),
            lang_hint: meta.lang.clone().or_else(|| meta.subtype.clone()),
            reason: meta.reason.clone(),
            subtype: meta.subtype.clone(),
        });

        cur_org = expected_end;
        cur_ann = end_end;

        if cur_ann < b_annot.len() {
            if take_start_id_at(annotated_text, cur_ann, &re_start_block, &re_start_short).is_none() {
                return Err(SegError::LeadingText(cur_ann));
            }
        }
    }

    if cur_ann != b_annot.len() {
        return Err(SegError::TrailingText(cur_ann));
    }
    if cur_org != bytes_orig.len() {
        return Err(SegError::ByteMismatch { id: -1, expected_start: cur_org, expected_end: bytes_orig.len(), model_len: 0 });
    }

    for i in 0..segs.len().saturating_sub(1) {
        assert!(segs[i].end == segs[i + 1].start, "non-continuous after id {}", segs[i].id);
    }

    Ok(segs)
}
