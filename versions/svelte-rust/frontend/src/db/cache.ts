import Dexie, { type Table } from "dexie";

export interface CachedImageBlob {
  url: string;
  blob: Blob;
}

export interface ChapterCacheEntry {
  id: string;
  providerId: string;
  chapterUrl: string;
  title: string;
  images: string[];
  nextUrl: string | null;
  imageBlobs?: CachedImageBlob[];
  createdAt: number;
  lastAccessedAt: number;
}

class ReaderDb extends Dexie {
  chapters!: Table<ChapterCacheEntry, string>;

  constructor() {
    super("mirror_komiku_v3");
    this.version(1).stores({
      chapters: "id,providerId,chapterUrl,lastAccessedAt,createdAt"
    });
  }
}

const db = new ReaderDb();

export function chapterCacheKey(providerId: string, chapterUrl: string): string {
  return `${providerId}:${normalizeChapterUrl(chapterUrl)}`;
}

export function normalizeChapterUrl(rawUrl: string): string {
  const value = rawUrl.trim();
  try {
    const parsed = new URL(value);
    parsed.hash = "";
    let path = parsed.pathname;
    if (!path.endsWith("/")) path += "/";
    parsed.pathname = path.replace(/\/{2,}/g, "/");
    return parsed.toString();
  } catch {
    return value;
  }
}

export async function getCachedChapter(
  providerId: string,
  chapterUrl: string
): Promise<ChapterCacheEntry | null> {
  const id = chapterCacheKey(providerId, chapterUrl);
  const row = await db.chapters.get(id);
  if (!row) return null;
  await db.chapters.update(id, { lastAccessedAt: Date.now() });
  return { ...row, lastAccessedAt: Date.now() };
}

export interface UpsertChapterInput {
  providerId: string;
  chapterUrl: string;
  title: string;
  images: string[];
  nextUrl: string | null;
  imageBlobs?: CachedImageBlob[];
}

export async function upsertChapter(input: UpsertChapterInput): Promise<void> {
  const normalizedUrl = normalizeChapterUrl(input.chapterUrl);
  const id = chapterCacheKey(input.providerId, normalizedUrl);
  const existing = await db.chapters.get(id);
  const now = Date.now();
  const row: ChapterCacheEntry = {
    id,
    providerId: input.providerId,
    chapterUrl: normalizedUrl,
    title: input.title,
    images: [...input.images],
    nextUrl: input.nextUrl,
    imageBlobs: input.imageBlobs ? [...input.imageBlobs] : existing?.imageBlobs,
    createdAt: existing?.createdAt ?? now,
    lastAccessedAt: now
  };
  await db.chapters.put(row);
}

export async function enforceLruLimit(maxEntries: number): Promise<void> {
  if (maxEntries <= 0) return;
  const count = await db.chapters.count();
  if (count <= maxEntries) return;
  const over = count - maxEntries;
  const victims = await db.chapters.orderBy("lastAccessedAt").limit(over).toArray();
  if (victims.length === 0) return;
  await db.chapters.bulkDelete(victims.map((item) => item.id));
}

export async function clearAllCache(): Promise<void> {
  await db.chapters.clear();
}

export async function getCacheCount(): Promise<number> {
  return db.chapters.count();
}
