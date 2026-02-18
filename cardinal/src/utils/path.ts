export type SplitPathResult = {
  name: string;
  directory: string;
};

export const splitPath = (path: string | undefined): SplitPathResult => {
  if (!path) {
    return { name: '', directory: '' };
  }

  if (path === '/') {
    return { name: '/', directory: '/' };
  }

  const slashIndex = path.lastIndexOf('/');
  if (slashIndex === -1) {
    return { name: path, directory: '' };
  }

  const directory = path.slice(0, slashIndex) || '/';
  const name = path.slice(slashIndex + 1);
  return { name, directory };
};
