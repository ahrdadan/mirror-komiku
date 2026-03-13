import { komikuProvider } from "./komiku/parser";
import type { ProviderParser } from "./types";

const PROVIDERS: ProviderParser[] = [komikuProvider];

export function findProviderById(providerId: string): ProviderParser | null {
  return PROVIDERS.find((provider) => provider.id === providerId) ?? null;
}

export function matchProviderByUrl(rawUrl: string): ProviderParser | null {
  try {
    const host = new URL(rawUrl).host.toLowerCase();
    return PROVIDERS.find((provider) => provider.matchesHost(host)) ?? null;
  } catch {
    return null;
  }
}

export function canonicalPath(providerId: string, encoded: string): string {
  return `/${providerId}/${encoded}`;
}
