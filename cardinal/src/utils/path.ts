export type SplitPathResult = {
  name: string;
  directory: string;
};

export const splitPath = (path: string | undefined): SplitPathResult => {
  if (!path) {
    return { name: '', directory: '' };
  }

  const normalized = path.replace(/\\/g, '/');
  if (normalized === '/') {
    return { name: '/', directory: '/' };
  }

  const slashIndex = normalized.lastIndexOf('/');
  if (slashIndex === -1) {
    return { name: normalized, directory: '' };
  }

  const directory = normalized.slice(0, slashIndex) || '/';
  const name = normalized.slice(slashIndex + 1) || normalized;
  return { name, directory };
};
