import { useCallback, useEffect, useState } from 'react';

const STORAGE_KEY = 'cardinal.ignorePaths';
const DEFAULT_IGNORE_PATHS = ['/Volumes'];

const readStoredValue = (): string[] | null => {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (!stored) return null;
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return null;
    const cleaned = parsed
      .filter((item) => typeof item === 'string')
      .map((item) => item.trim())
      .filter((item) => item.length > 0);
    return cleaned.length > 0 ? cleaned : [];
  } catch (error) {
    console.warn('Unable to read saved ignore paths', error);
    return null;
  }
};

export function useIgnorePaths() {
  const [ignorePaths, setIgnorePathsState] = useState<string[]>(
    () => readStoredValue() ?? DEFAULT_IGNORE_PATHS,
  );

  useEffect(() => {
    const stored = readStoredValue();
    if (stored) return;
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(DEFAULT_IGNORE_PATHS));
    } catch (error) {
      console.warn('Unable to persist default ignore paths', error);
    }
  }, []);

  const setIgnorePaths = useCallback((next: string[]) => {
    const cleaned = next.map((item) => item.trim()).filter((item) => item.length > 0);
    setIgnorePathsState(cleaned);
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(cleaned));
    } catch (error) {
      console.warn('Unable to persist ignore paths', error);
    }
  }, []);

  return { ignorePaths, setIgnorePaths, defaultIgnorePaths: DEFAULT_IGNORE_PATHS };
}
