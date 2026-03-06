export const isPathInputValid = (input: string): boolean => {
  const trimmed = input.trim();
  if (trimmed.length === 0) return false;
  if (trimmed.startsWith('/')) return true;
  return trimmed === '~' || trimmed.startsWith('~/');
};

type WatchRootValidation = {
  isValid: boolean;
  errorKey: 'watchRoot.errors.required' | 'watchRoot.errors.absolute' | null;
};

export const getWatchRootValidation = (input: string): WatchRootValidation => {
  const trimmed = input.trim();
  if (trimmed.length === 0) {
    return { isValid: false, errorKey: 'watchRoot.errors.required' };
  }
  if (!isPathInputValid(trimmed)) {
    return { isValid: false, errorKey: 'watchRoot.errors.absolute' };
  }
  return { isValid: true, errorKey: null };
};
