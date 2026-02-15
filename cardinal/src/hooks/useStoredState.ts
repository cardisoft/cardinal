import { useCallback, useState } from 'react';

type UseStoredStateOptions<T> = {
  key: string;
  defaultValue: T;
  read: (raw: string) => T | null;
  write: (value: T) => string;
  normalize?: (value: T) => T;
  readErrorMessage: string;
  writeErrorMessage: string;
};

export function useStoredState<T>({
  key,
  defaultValue,
  read,
  write,
  normalize,
  readErrorMessage,
  writeErrorMessage,
}: UseStoredStateOptions<T>): [T, (next: T) => void] {
  const [state, setState] = useState<T>(() => {
    if (typeof window === 'undefined') {
      return defaultValue;
    }

    try {
      const raw = window.localStorage.getItem(key);
      if (raw != null) {
        const parsed = read(raw);
        if (parsed != null) {
          return parsed;
        }
      }
    } catch (error) {
      console.warn(readErrorMessage, error);
    }

    try {
      window.localStorage.setItem(key, write(defaultValue));
    } catch (error) {
      console.warn(writeErrorMessage, error);
    }

    return defaultValue;
  });

  const setStoredState = useCallback(
    (next: T) => {
      const normalized = normalize ? normalize(next) : next;
      setState(normalized);

      if (typeof window === 'undefined') {
        return;
      }

      try {
        window.localStorage.setItem(key, write(normalized));
      } catch (error) {
        console.warn(writeErrorMessage, error);
      }
    },
    [key, normalize, write, writeErrorMessage],
  );

  return [state, setStoredState];
}
