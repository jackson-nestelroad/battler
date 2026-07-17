export function formatUuid(id: string): string {
  const cleaned = id.replace(/-/g, "").toLowerCase();
  if (cleaned.length !== 32) return id;
  return `${cleaned.slice(0, 8)}-${cleaned.slice(8, 12)}-${cleaned.slice(12, 16)}-${cleaned.slice(16, 20)}-${cleaned.slice(20)}`;
}
