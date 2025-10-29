// file: src/chunking.rs

use regex::Regex;
use lazy_static::lazy_static;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Header,
    CodeFence,
    List,
    Table,
    Quote,
    Paragraph,
    Hr,
    Blank,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub start: usize,
    pub end: usize,
    pub lang: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub kind_summary: Vec<BlockKind>,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ChunkingConfig {
    pub max_tokens: usize,
    pub overlap_tokens: usize,
    pub approx_chars_per_token: f32,
    pub hard_max_bytes: usize,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_tokens: 400,
            overlap_tokens: 60,
            approx_chars_per_token: 4.0,
            hard_max_bytes: 64 * 1024,
        }
    }
}

fn approx_tokens(s: &str, cpt: f32) -> usize {
    ((s.chars().count() as f32) / cpt).ceil() as usize
}

lazy_static! {
    static ref RE_HEADER: Regex = Regex::new(r"(?m)^(?P<hash>#{1,6})\s+(.+?)\s*$").unwrap();
    static ref RE_CODE_FENCE_OPEN: Regex = Regex::new(r"(?m)^```([A-Za-z0-9_+-]+)?\s*$").unwrap();
    static ref RE_CODE_FENCE_CLOSE: Regex = Regex::new(r"(?m)^```\s*$").unwrap();
    static ref RE_LIST: Regex = Regex::new(r#"(?m)^(?:\s{0,3}(?:[-+*]|\d{1,3}[.)]))\s+"#).unwrap();
    static ref RE_TABLE_ROW: Regex = Regex::new(r"(?m)^\s*\|.+\|\s*$").unwrap();
    static ref RE_TABLE_SEP: Regex = Regex::new(r"(?m)^\s*\|\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\s*\|?\s*$").unwrap();
    static ref RE_QUOTE: Regex = Regex::new(r"(?m)^\s*>\s+").unwrap();
    static ref RE_HR: Regex = Regex::new(r"(?m)^\s*(?:[-]{3,}|[_]{3,}|[*]{3,})\s*$").unwrap();
    static ref RE_BLANK: Regex = Regex::new(r"(?m)^\s*$").unwrap();
}

pub fn parse_blocks(input: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;
    let bytes = input.as_bytes();
    let len = bytes.len();

    while i < len {
        // Find line boundaries
        let line_start = i;
        let mut line_end = i;
        while line_end < len && bytes[line_end] != b'\n' {
            line_end += 1;
        }
        
        // Get the line as &str (safe because we're working with line boundaries)
        let line = if line_end <= len {
            &input[line_start..line_end]
        } else {
            &input[line_start..]
        };

        // Code fence
        if RE_CODE_FENCE_OPEN.is_match(line) {
            let fence_start = line_start;
            
            // Skip opening fence line
            i = if line_end < len { line_end + 1 } else { line_end };
            
            // Find closing fence
            while i < len {
                let fence_line_start = i;
                let mut fence_line_end = i;
                while fence_line_end < len && bytes[fence_line_end] != b'\n' {
                    fence_line_end += 1;
                }
                
                let fence_line = &input[fence_line_start..fence_line_end];
                if RE_CODE_FENCE_CLOSE.is_match(fence_line) {
                    i = if fence_line_end < len { fence_line_end + 1 } else { fence_line_end };
                    break;
                }
                
                i = if fence_line_end < len { fence_line_end + 1 } else { fence_line_end };
            }
            
            blocks.push(Block {
                kind: BlockKind::CodeFence,
                start: fence_start,
                end: i.saturating_sub(1).max(fence_start),
                lang: None,
            });
            continue;
        }

        // Table - group consecutive table rows
        if RE_TABLE_ROW.is_match(line) || RE_TABLE_SEP.is_match(line) {
            let table_start = line_start;
            
            while i < len {
                let t_line_start = i;
                let mut t_line_end = i;
                while t_line_end < len && bytes[t_line_end] != b'\n' {
                    t_line_end += 1;
                }
                
                let t_line = &input[t_line_start..t_line_end];
                if !RE_TABLE_ROW.is_match(t_line) && !RE_TABLE_SEP.is_match(t_line) {
                    break;
                }
                
                i = if t_line_end < len { t_line_end + 1 } else { t_line_end };
            }
            
            blocks.push(Block {
                kind: BlockKind::Table,
                start: table_start,
                end: i.saturating_sub(1).max(table_start),
                lang: None,
            });
            continue;
        }

        // List - group consecutive list items
        if RE_LIST.is_match(line) {
            let list_start = line_start;
            
            while i < len {
                let l_line_start = i;
                let mut l_line_end = i;
                while l_line_end < len && bytes[l_line_end] != b'\n' {
                    l_line_end += 1;
                }
                
                let l_line = &input[l_line_start..l_line_end];
                // Continue if list item or indented continuation
                if RE_LIST.is_match(l_line) || (l_line.starts_with("  ") && !l_line.trim().is_empty()) {
                    i = if l_line_end < len { l_line_end + 1 } else { l_line_end };
                } else {
                    break;
                }
            }
            
            blocks.push(Block {
                kind: BlockKind::List,
                start: list_start,
                end: i.saturating_sub(1).max(list_start),
                lang: None,
            });
            continue;
        }

        // Quote - group consecutive quote lines
        if RE_QUOTE.is_match(line) {
            let quote_start = line_start;
            
            while i < len {
                let q_line_start = i;
                let mut q_line_end = i;
                while q_line_end < len && bytes[q_line_end] != b'\n' {
                    q_line_end += 1;
                }
                
                let q_line = &input[q_line_start..q_line_end];
                if !RE_QUOTE.is_match(q_line) {
                    break;
                }
                
                i = if q_line_end < len { q_line_end + 1 } else { q_line_end };
            }
            
            blocks.push(Block {
                kind: BlockKind::Quote,
                start: quote_start,
                end: i.saturating_sub(1).max(quote_start),
                lang: None,
            });
            continue;
        }

        // Header
        if RE_HEADER.is_match(line) {
            blocks.push(Block {
                kind: BlockKind::Header,
                start: line_start,
                end: line_end,
                lang: None,
            });
            i = if line_end < len { line_end + 1 } else { line_end };
            continue;
        }

        // HR - skip
        if RE_HR.is_match(line) {
            i = if line_end < len { line_end + 1 } else { line_end };
            continue;
        }

        // Blank line - skip
        if line.trim().is_empty() {
            i = if line_end < len { line_end + 1 } else { line_end };
            continue;
        }

        // Paragraph - group consecutive non-blank lines
        if !line.trim().is_empty() {
            let para_start = line_start;
            
            while i < len {
                let p_line_start = i;
                let mut p_line_end = i;
                while p_line_end < len && bytes[p_line_end] != b'\n' {
                    p_line_end += 1;
                }
                
                let p_line = &input[p_line_start..p_line_end];
                
                // Stop at special blocks or blank lines
                if p_line.trim().is_empty()
                    || RE_HEADER.is_match(p_line)
                    || RE_CODE_FENCE_OPEN.is_match(p_line)
                    || RE_CODE_FENCE_CLOSE.is_match(p_line)
                    || RE_LIST.is_match(p_line)
                    || RE_TABLE_ROW.is_match(p_line)
                    || RE_QUOTE.is_match(p_line)
                    || RE_HR.is_match(p_line)
                {
                    break;
                }
                
                i = if p_line_end < len { p_line_end + 1 } else { p_line_end };
            }
            
            blocks.push(Block {
                kind: BlockKind::Paragraph,
                start: para_start,
                end: i.saturating_sub(1).max(para_start),
                lang: None,
            });
            continue;
        }

        i = if line_end < len { line_end + 1 } else { line_end };
    }

    blocks
}

pub fn make_chunks(doc_id: &str, input: &str, blocks: &[Block], cfg: &ChunkingConfig) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let min_chunk_size = 30; // Минимальный размер чанка в символах

    for b in blocks {
        let text = &input[b.start..b.end];
        
        // Skip too small chunks
        if text.trim().len() < min_chunk_size {
            continue;
        }

        // Check if chunk is too large
        let tokens = approx_tokens(text, cfg.approx_chars_per_token);
        
        if tokens <= cfg.max_tokens {
            // Chunk fits, create single chunk
            let id = chunk_id(doc_id, b.start, b.end, text);
            chunks.push(Chunk {
                id,
                kind_summary: vec![b.kind],
                start: b.start,
                end: b.end,
                text: text.to_string(),
            });
        } else {
            // Chunk too large, split by sentences
            let sentences = split_sentences(text);
            let mut current_chunk_parts: Vec<&str> = Vec::new();
            let mut current_chunk_start_offset = 0;

            for sent in sentences {
                let current_text = current_chunk_parts.join("");
                let would_be_tokens = approx_tokens(&current_text, cfg.approx_chars_per_token) 
                    + approx_tokens(sent, cfg.approx_chars_per_token);
                
                if would_be_tokens > cfg.max_tokens && !current_chunk_parts.is_empty() {
                    // Save current chunk
                    let chunk_text = current_chunk_parts.join("");
                    if chunk_text.trim().len() >= min_chunk_size {
                        let chunk_start = b.start + current_chunk_start_offset;
                        let chunk_end = chunk_start + chunk_text.len();
                        let id = chunk_id(doc_id, chunk_start, chunk_end, &chunk_text);
                        chunks.push(Chunk {
                            id,
                            kind_summary: vec![b.kind],
                            start: chunk_start,
                            end: chunk_end,
                            text: chunk_text.clone(),
                        });
                        
                        // Update offset for next chunk
                        current_chunk_start_offset += chunk_text.len();
                    }
                    
                    // Start new chunk with current sentence
                    current_chunk_parts.clear();
                }
                
                current_chunk_parts.push(sent);
            }

            // Save last chunk
            if !current_chunk_parts.is_empty() {
                let chunk_text = current_chunk_parts.join("");
                if chunk_text.trim().len() >= min_chunk_size {
                    let chunk_start = b.start + current_chunk_start_offset;
                    let chunk_end = chunk_start + chunk_text.len();
                    let id = chunk_id(doc_id, chunk_start, chunk_end, &chunk_text);
                    chunks.push(Chunk {
                        id,
                        kind_summary: vec![b.kind],
                        start: chunk_start,
                        end: chunk_end,
                        text: chunk_text,
                    });
                }
            }
        }
    }

    chunks
}

fn split_sentences(paragraph: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let bytes = paragraph.as_bytes();
    for (i, ch) in paragraph.char_indices() {
        match ch {
            '.' | '!' | '?' | '…' => {
                let mut j = i + ch.len_utf8();
                while j < bytes.len() {
                    let cj = paragraph[j..].chars().next().unwrap();
                    if cj.is_whitespace() || matches!(cj, '"' | '\'' | ')' | '\u{00BB}' | '\u{201C}' | '\u{2019}') {
                        j += cj.len_utf8();
                    } else {
                        break;
                    }
                }
                let sent = &paragraph[start..j];
                if !sent.trim().is_empty() {
                    out.push(sent);
                }
                start = j;
            }
            _ => {}
        }
    }
    if start < paragraph.len() {
        let tail = &paragraph[start..];
        if !tail.trim().is_empty() {
            out.push(tail);
        }
    }
    out
}

fn chunk_id(doc_id: &str, start: usize, end: usize, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}:{}:{}", doc_id, start, end, text).as_bytes());
    let digest = hasher.finalize();
    format!("chunk::{:x}", digest)
}

pub fn chunk_document(doc_id: &str, input: &str, cfg: &ChunkingConfig) -> Vec<Chunk> {
    let blocks = parse_blocks(input);
    make_chunks(doc_id, input, &blocks, cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic() {
        let doc = "# Заголовок\n\nПараграф текста.";
        let cfg = ChunkingConfig::default();
        let ch = chunk_document("doc::1", doc, &cfg);
        assert!(!ch.is_empty());
    }
}