// Format bytes into KB with one decimal place (legacy function)
export function formatKB(bytes: number | null | undefined): string | null {
  if (bytes == null || !Number.isFinite(bytes)) return null;
  const kb = bytes / 1024;
  return `${kb.toFixed(kb < 10 ? 1 : 0)} KB`;
}

// Format timestamp (in seconds) as YYYY-MM-DD HH:mm:ss
export function formatTimestamp(timestampSec: number | null | undefined): string | null {
  if (timestampSec == null || !Number.isFinite(timestampSec)) return null;
  const date = new Date(timestampSec * 1000);

  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  const seconds = String(date.getSeconds()).padStart(2, '0');

  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
}

const SIZE_UNITS = ['B', 'KB', 'MB', 'G', 'T', 'P'] as const;

/** Trims trailing zeros so 1.20 becomes 1.2 and 4.00 becomes 4. */
const trimZeros = (value: string): string =>
  value.includes('.') ? value.replace(/\.?0+$/, '') : value;

/**
 * Human-readable byte size: `4KB`, `50MB`, `200MB`, `1G`, `1.24G`.
 * Two decimals below 10 (so small differences stay visible), none above.
 */
export function formatSize(bytes: number | null | undefined): string | null {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) {
    return null;
  }
  if (bytes < 1024) {
    return `${Math.round(bytes)}${SIZE_UNITS[0]}`;
  }

  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < SIZE_UNITS.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const formatted = value < 10 ? trimZeros(value.toFixed(2)) : String(Math.round(value));
  return `${formatted}${SIZE_UNITS[unitIndex]}`;
}
