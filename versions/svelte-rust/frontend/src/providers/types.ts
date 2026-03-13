export interface ParsedChapter {
  sourceUrl: string;
  title: string;
  imageUrls: string[];
  nextUrl: string | null;
}

export interface ProviderParser {
  id: string;
  label: string;
  matchesHost: (host: string) => boolean;
  parseChapter: (html: string, sourceUrl: string) => ParsedChapter;
}
