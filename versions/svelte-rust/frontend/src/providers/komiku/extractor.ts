import type { ParsedChapter } from "../types";
import {
  IMAGE_ATTR_PRIORITY,
  IMAGE_SELECTORS,
  NEXT_SELECTORS,
  TITLE_SELECTORS
} from "./selectors";

function resolveAbsoluteUrl(baseUrl: string, rawUrl: string): string | null {
  try {
    return new URL(rawUrl, baseUrl).href;
  } catch {
    return null;
  }
}

function extractTitle(doc: Document): string {
  for (const selector of TITLE_SELECTORS) {
    const element = doc.querySelector(selector);
    if (!element) continue;
    const text = element.textContent?.trim();
    if (text) return text;
  }
  return "Manga Chapter";
}

function extractImageUrls(doc: Document, sourceUrl: string): string[] {
  const output: string[] = [];
  const seen = new Set<string>();

  for (const selector of IMAGE_SELECTORS) {
    const nodes = Array.from(doc.querySelectorAll(selector));
    for (const node of nodes) {
      let raw = "";
      for (const attr of IMAGE_ATTR_PRIORITY) {
        const value = (node.getAttribute(attr) ?? "").trim();
        if (value) {
          raw = value;
          break;
        }
      }
      if (!raw || raw.startsWith("data:")) continue;
      const absolute = resolveAbsoluteUrl(sourceUrl, raw);
      if (!absolute || seen.has(absolute)) continue;
      seen.add(absolute);
      output.push(absolute);
    }

    if (output.length > 0) {
      return output;
    }
  }

  return output;
}

function extractChapterNumber(pathname: string): number | null {
  const lower = String(pathname).toLowerCase();
  const marker = "chapter-";
  const markerIndex = lower.indexOf(marker);
  if (markerIndex < 0) return null;
  let digits = "";
  for (let i = markerIndex + marker.length; i < lower.length; i += 1) {
    const ch = lower[i];
    if (ch >= "0" && ch <= "9") {
      digits += ch;
      continue;
    }
    break;
  }
  if (!digits) return null;
  const value = Number(digits);
  return Number.isFinite(value) ? value : null;
}

function extractNextUrl(doc: Document, sourceUrl: string): string | null {
  const candidates: string[] = [];
  let currentChapter: number | null = null;
  try {
    currentChapter = extractChapterNumber(new URL(sourceUrl).pathname);
  } catch {
    currentChapter = null;
  }

  for (const selector of NEXT_SELECTORS) {
    const nodes = Array.from(doc.querySelectorAll(selector));
    for (const node of nodes) {
      const href = (node.getAttribute("href") ?? "").trim();
      if (!href || href === "#" || href.startsWith("javascript:")) continue;

      const absolute = resolveAbsoluteUrl(sourceUrl, href);
      if (!absolute || absolute === sourceUrl) continue;

      const rel = (node.getAttribute("rel") ?? "").toLowerCase();
      const text = (node.textContent ?? "").trim().toLowerCase();
      if (
        rel.includes("next") ||
        text.includes("next") ||
        text.includes("selanjutnya")
      ) {
        return absolute;
      }

      candidates.push(absolute);
    }
    if (candidates.length > 0) break;
  }

  if (currentChapter !== null) {
    const expected = `chapter-${currentChapter + 1}`;
    const exact = candidates.find((candidate) =>
      candidate.toLowerCase().includes(expected)
    );
    if (exact) {
      return exact;
    }
  }

  const chapterCandidates = candidates.filter((candidate) =>
    candidate.toLowerCase().includes("chapter-")
  );
  return chapterCandidates.length > 0
    ? chapterCandidates[chapterCandidates.length - 1]
    : null;
}

export function parseKomikuChapter(html: string, sourceUrl: string): ParsedChapter {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const title = extractTitle(doc);
  const imageUrls = extractImageUrls(doc, sourceUrl);
  if (imageUrls.length === 0) {
    throw new Error("image extraction returned zero URLs");
  }
  const nextUrl = extractNextUrl(doc, sourceUrl);
  return {
    sourceUrl,
    title,
    imageUrls,
    nextUrl
  };
}
