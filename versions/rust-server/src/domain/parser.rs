use std::collections::HashSet;

use anyhow::{anyhow, Result};
use scraper::{Html as ScraperHtml, Selector};
use url::Url;

use crate::domain::models::ParsedChapter;

pub fn parse_chapter_html(source_html: &str, source_url: &Url) -> Result<ParsedChapter> {
    let doc = ScraperHtml::parse_document(source_html);

    let title = select_title(&doc).unwrap_or_else(|| "Manga Chapter".to_string());
    let image_urls = select_image_urls(&doc, source_url)?;
    let next_url = select_next_url(&doc, source_url);

    Ok(ParsedChapter {
        title,
        image_urls,
        next_url,
    })
}

fn select_title(doc: &ScraperHtml) -> Option<String> {
    let selectors = ["div header h1", "h1"];
    for sel in selectors {
        if let Ok(selector) = Selector::parse(sel) {
            if let Some(el) = doc.select(&selector).next() {
                let text = el.text().collect::<Vec<_>>().join(" ");
                let clean = text.trim();
                if !clean.is_empty() {
                    return Some(clean.to_string());
                }
            }
        }
    }
    None
}

fn select_image_urls(doc: &ScraperHtml, source_url: &Url) -> Result<Vec<Url>> {
    let selectors = [
        "#Baca_Komik img[src]",
        "div#Baca_Komik img[src]",
        "img[src]",
    ];
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for sel in selectors {
        let Ok(selector) = Selector::parse(sel) else {
            continue;
        };

        for node in doc.select(&selector) {
            let Some(raw_src) = node.value().attr("src") else {
                continue;
            };
            let src = raw_src.trim();
            if src.is_empty() || src.starts_with("data:") {
                continue;
            }
            let Ok(url) = source_url.join(src) else {
                continue;
            };
            let key = url.as_str().to_string();
            if seen.insert(key) {
                out.push(url);
            }
        }

        if !out.is_empty() {
            break;
        }
    }

    if out.is_empty() {
        return Err(anyhow!("image extraction returned zero URLs"));
    }

    Ok(out)
}

fn select_next_url(doc: &ScraperHtml, source_url: &Url) -> Option<Url> {
    let selectors = [
        "a[rel='next']",
        "a.next",
        ".next a",
        ".navig a",
        ".pagination a",
        "a",
    ];
    let mut candidates = Vec::new();
    let current_chapter = extract_chapter_number(source_url.path());

    for sel in selectors {
        let Ok(selector) = Selector::parse(sel) else {
            continue;
        };

        for node in doc.select(&selector) {
            let Some(raw_href) = node.value().attr("href") else {
                continue;
            };
            let href = raw_href.trim();
            if href.is_empty() || href == "#" || href.starts_with("javascript:") {
                continue;
            }
            let Ok(url) = source_url.join(href) else {
                continue;
            };
            if url == *source_url {
                continue;
            }
            let text = node
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_ascii_lowercase();
            let rel = node
                .value()
                .attr("rel")
                .unwrap_or_default()
                .to_ascii_lowercase();

            if rel.contains("next") || text.contains("next") || text.contains("selanjutnya") {
                return Some(url);
            }
            candidates.push(url);
        }

        if !candidates.is_empty() {
            break;
        }
    }

    if let Some(current) = current_chapter {
        let wanted = format!("chapter-{}", current + 1);
        if let Some(found) = candidates
            .iter()
            .find(|u| u.path().to_ascii_lowercase().contains(&wanted))
        {
            return Some(found.clone());
        }
    }

    candidates
        .into_iter()
        .filter(|u| u.path().to_ascii_lowercase().contains("chapter-"))
        .last()
}

fn extract_chapter_number(path: &str) -> Option<i64> {
    let lower = path.to_ascii_lowercase();
    let marker = "chapter-";
    let idx = lower.find(marker)?;
    let digits = lower[idx + marker.len()..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    digits.parse::<i64>().ok()
}
