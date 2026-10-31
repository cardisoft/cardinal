/**
 * Parses a `name,search` CSV payload into filter entries.
 * A header row is optional so exported files and hand-written lists both work.
 */
export const parseFilterRows = (rows: string[][]): { name: string; search: string }[] => {
  if (rows.length === 0) {
    return [];
  }
  const header = rows[0].map((cell) => cell.trim().toLowerCase());
  const body = header[0] === 'name' && header[1] === 'search' ? rows.slice(1) : rows;
  return body
    .map((row) => ({ name: (row[0] ?? '').trim(), search: (row[1] ?? '').trim() }))
    .filter((entry) => entry.name.length > 0);
};
