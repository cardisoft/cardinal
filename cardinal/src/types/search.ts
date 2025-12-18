export type SearchResultMetadata = Readonly<{
  type: number;
  size: number;
  mtime: number;
  ctime: number;
}>;

export type SearchResultItem = Readonly<{
  path: string;
  metadata?: SearchResultMetadata;
  size?: number;
  mtime?: number;
  ctime?: number;
  icon?: string;
  iconWidth?: number;
  iconHeight?: number;
}>;

export type NodeInfoResponse = Readonly<{
  path: string;
  icon?: string | null;
  iconWidth?: number | null;
  iconHeight?: number | null;
  metadata?: SearchResultMetadata | null;
  size?: number | null;
  mtime?: number | null;
  ctime?: number | null;
}>;
