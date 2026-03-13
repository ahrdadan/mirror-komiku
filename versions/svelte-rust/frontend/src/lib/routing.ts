export type ParsedRoute =
  | { kind: "home" }
  | { kind: "raw-url"; rawUrl: string }
  | { kind: "chapter"; providerId: string; encoded: string };

export function parsePath(pathname: string): ParsedRoute {
  if (!pathname || pathname === "/" || pathname === "/index.html") {
    return { kind: "home" };
  }

  const trimmed = pathname.startsWith("/") ? pathname.slice(1) : pathname;
  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    return { kind: "raw-url", rawUrl: trimmed };
  }

  const segments = trimmed.split("/").filter(Boolean);
  if (segments.length >= 2) {
    return {
      kind: "chapter",
      providerId: segments[0],
      encoded: segments[1]
    };
  }

  return { kind: "home" };
}
