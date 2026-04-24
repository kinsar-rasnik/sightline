export function formatUnixSeconds(secs: number | null | undefined): string {
  if (!secs) return "–";
  const d = new Date(secs * 1000);
  return d.toISOString().replace("T", " ").slice(0, 19) + " UTC";
}

export function formatRelative(secsAhead: number | null | undefined): string {
  if (secsAhead == null) return "–";
  if (secsAhead <= 0) return "now";
  const minutes = Math.round(secsAhead / 60);
  if (minutes < 1) return `${secsAhead}s`;
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.round(hours / 24);
  return `${days}d`;
}

export function formatDurationSeconds(secs: number): string {
  if (secs <= 0) return "0s";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  const parts: string[] = [];
  if (h) parts.push(`${h}h`);
  if (m) parts.push(`${m}m`);
  if (s || parts.length === 0) parts.push(`${s}s`);
  return parts.join(" ");
}
