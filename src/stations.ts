export interface Station {
  id: string;
  name: string;
  baseUrl: string;
}
export interface StationPreferences {
  version: 1;
  selected: string;
  custom: Station[];
}
export const DEFAULT_STATION: Station = {
  id: "tokentoapi",
  name: "TokenToAPI",
  baseUrl: "https://tokentoapi.com",
};
export const STATION_STORAGE_KEY = "tokentoapi-connect.stations.v1";

export function normalizeStationUrl(value: string): string {
  if (value.trim().length > 2048)
    throw new Error("API address is too long. Check it and try again.");
  let url: URL;
  try {
    url = new URL(value.trim());
  } catch {
    throw new Error("Enter a full API address, e.g. https://api.example.com.");
  }
  const local = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
  if (url.protocol !== "https:" && !(url.protocol === "http:" && local)) {
    throw new Error("Use an HTTPS address; HTTP is allowed for local development.");
  }
  if (!url.hostname || url.username || url.password || url.search || url.hash) {
    throw new Error("The address can't include a username, password, query params, or anchor.");
  }
  const path = url.pathname.replace(/\/+$/, "");
  if (/\/(responses|messages|chat\/completions)$/.test(path)) {
    throw new Error(
      "Enter the API base address, without endpoint paths like /responses or /messages.",
    );
  }
  url.pathname = path.replace(/\/v1$/, "");
  return url.toString().replace(/\/+$/, "");
}
export function stationAddress(station: Station): string {
  return station.baseUrl.replace(/^https?:\/\//, "");
}
function emptyPreferences(): StationPreferences {
  return { version: 1, selected: DEFAULT_STATION.id, custom: [] };
}
// Persist names and public API addresses only, never keys or imported config.
export function parseStationPreferences(
  raw: string | null,
): StationPreferences {
  const fallback = emptyPreferences();
  if (!raw) return fallback;
  try {
    const value: unknown = JSON.parse(raw);
    if (!value || typeof value !== "object") return fallback;
    const data = value as Record<string, unknown>;
    if (data.version !== 1 || !Array.isArray(data.custom)) return fallback;
    const seenIds = new Set([DEFAULT_STATION.id]);
    const seenUrls = new Set([DEFAULT_STATION.baseUrl]);
    for (const row of data.custom.slice(0, 30)) {
      if (!row || typeof row !== "object") continue;
      const { id, name, baseUrl } = row;
      if (
        typeof id !== "string" ||
        !/^custom-[a-zA-Z0-9-]{1,80}$/.test(id) ||
        seenIds.has(id) ||
        typeof name !== "string" ||
        !name.trim() ||
        name.trim().length > 32 ||
        /[\u0000-\u001f\u007f]/.test(name) ||
        typeof baseUrl !== "string"
      )
        continue;
      try {
        const normalized = normalizeStationUrl(baseUrl);
        if (seenUrls.has(normalized)) continue;
        fallback.custom.push({ id, name: name.trim(), baseUrl: normalized });
        seenIds.add(id);
        seenUrls.add(normalized);
      } catch {
        /* Ignore malformed entries without blocking the app. */
      }
    }
    if (typeof data.selected === "string" && seenIds.has(data.selected))
      fallback.selected = data.selected;
    return fallback;
  } catch {
    return fallback;
  }
}
export function loadStationPreferences(): StationPreferences {
  try {
    return parseStationPreferences(localStorage.getItem(STATION_STORAGE_KEY));
  } catch {
    return emptyPreferences();
  }
}
export function saveStationPreferences(
  preferences: StationPreferences,
): boolean {
  try {
    const safe = parseStationPreferences(JSON.stringify(preferences));
    localStorage.setItem(STATION_STORAGE_KEY, JSON.stringify(safe));
    return true;
  } catch {
    return false;
  }
}
