// Minimal RFC 4180 CSV encode/decode used by the bookmark and filter import/export flows.
// A hand-rolled implementation keeps the desktop bundle free of an extra dependency.

const needsQuoting = (value: string): boolean => /[",\r\n]/.test(value);

const escapeField = (value: string): string =>
  needsQuoting(value) ? `"${value.replace(/"/g, '""')}"` : value;

/** Encodes rows into CSV text. A trailing newline is added when there is at least one row. */
export const encodeCsv = (rows: readonly (readonly string[])[]): string => {
  if (rows.length === 0) {
    return '';
  }
  return `${rows.map((row) => row.map(escapeField).join(',')).join('\r\n')}\r\n`;
};

/**
 * Decodes CSV text into rows. Handles quoted fields, escaped quotes (`""`), CRLF/LF line
 * endings, and a leading UTF-8 BOM (Excel and Numbers both write one).
 */
export const decodeCsv = (text: string): string[][] => {
  const input = text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  const rows: string[][] = [];
  let row: string[] = [];
  let field = '';
  let inQuotes = false;
  let i = 0;

  const endField = () => {
    row.push(field);
    field = '';
  };
  const endRow = () => {
    endField();
    rows.push(row);
    row = [];
  };

  while (i < input.length) {
    const char = input[i];

    if (inQuotes) {
      if (char === '"') {
        if (input[i + 1] === '"') {
          field += '"';
          i += 2;
          continue;
        }
        inQuotes = false;
        i += 1;
        continue;
      }
      field += char;
      i += 1;
      continue;
    }

    if (char === '"') {
      inQuotes = true;
      i += 1;
      continue;
    }
    if (char === ',') {
      endField();
      i += 1;
      continue;
    }
    if (char === '\r') {
      // Treat CRLF and a bare CR as one line break.
      endRow();
      i += input[i + 1] === '\n' ? 2 : 1;
      continue;
    }
    if (char === '\n') {
      endRow();
      i += 1;
      continue;
    }

    field += char;
    i += 1;
  }

  // Flush the final field/row unless the input ended exactly on a line break.
  if (field.length > 0 || row.length > 0) {
    endRow();
  }

  return rows;
};
