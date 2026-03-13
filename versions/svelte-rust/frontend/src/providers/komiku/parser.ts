import { parseKomikuChapter } from "./extractor";
import type { ProviderParser } from "../types";

export const komikuProvider: ProviderParser = {
  id: "komiku",
  label: "Komiku",
  matchesHost(host) {
    const lower = host.toLowerCase();
    return lower === "komiku.org" || lower.endsWith(".komiku.org");
  },
  parseChapter(html, sourceUrl) {
    return parseKomikuChapter(html, sourceUrl);
  }
};
