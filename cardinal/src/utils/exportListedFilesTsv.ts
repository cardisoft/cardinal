import { formatKB, formatTimestamp } from './format';
import { splitPath } from './path';
import type { NodeInfoResponse } from '../types/search';

export type TsvColumnLabels = Readonly<{
  filename: string;
  path: string;
  size: string;
  modified: string;
  created: string;
}>;

const sanitizeField = (value: string): string => value.replace(/[\t\r\n]+/g, ' ');
const MAX_FILENAME_TERM_LENGTH = 48;

const toDisplayFields = (node: NodeInfoResponse): [string, string, string, string, string] => {
  const path = node.path ?? '';
  const pathParts = splitPath(path);
  const filename = path === '/' ? '' : pathParts.name;
  const directoryPath = pathParts.directory;

  const sizeBytes = node.metadata?.size ?? node.size;
  const mtimeSec = node.metadata?.mtime ?? node.mtime;
  const ctimeSec = node.metadata?.ctime ?? node.ctime;
  const isDirectory = (node.metadata?.type ?? -1) === 1;

  const sizeText = !isDirectory ? formatKB(sizeBytes) : null;
  const mtimeText = formatTimestamp(mtimeSec);
  const ctimeText = formatTimestamp(ctimeSec);

  return [
    sanitizeField(filename),
    sanitizeField(directoryPath),
    sanitizeField(sizeText ?? '—'),
    sanitizeField(mtimeText ?? '—'),
    sanitizeField(ctimeText ?? '—'),
  ];
};

export const buildListedFilesTsvHeader = (labels: TsvColumnLabels): string => {
  return [
    sanitizeField(labels.filename),
    sanitizeField(labels.path),
    sanitizeField(labels.size),
    sanitizeField(labels.modified),
    sanitizeField(labels.created),
  ].join('\t');
};

export const buildListedFilesTsvRows = (nodes: readonly NodeInfoResponse[]): string => {
  return nodes.map((node) => toDisplayFields(node).join('\t')).join('\n');
};

export const buildListedFilesTsv = (
  nodes: readonly NodeInfoResponse[],
  labels: TsvColumnLabels,
): string => {
  const header = buildListedFilesTsvHeader(labels);
  const body = buildListedFilesTsvRows(nodes);
  if (!body) {
    return header;
  }
  return `${header}\n${body}`;
};

const toFilenameTerm = (query: string): string => {
  const normalized = query
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/-{2,}/g, '-')
    .replace(/^-+|-+$/g, '');
  if (!normalized) {
    return 'all-files';
  }
  return normalized.slice(0, MAX_FILENAME_TERM_LENGTH);
};

export const createListedFilesTsvFilename = (query: string, now: Date = new Date()): string => {
  const dateStamp = now.toISOString().slice(0, 10);
  const term = toFilenameTerm(query);
  return `cardinal-word-list-${term}-${dateStamp}.tsv`;
};
