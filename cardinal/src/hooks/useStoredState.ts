import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

type UseStoredStateOptions<T> = {
  key: string;
  defaultValue: T;
  read: (raw: string) => T | null;
  write: (value: T) => string;
  normalize?: (value: T) => T;
  readErrorMessage: string;
  writeErrorMessage: string;
  /**
   * Tauri event that signals "another window rewrote this key". When provided, the hook
   * re-reads localStorage on every delivery, which is how the separate bookmarks window and
   * the main window stay in sync (they share the webview data store but not React state).
   */
  syncEvent?: string;
};

export function useStoredState<T>({
  key,
  defaultValue,
  read,
  write,
  normalize,
  readErrorMessage,
  writeErrorMessage,
  syncEvent,
}: UseStoredStateOptions<T>): [T, (next: T | ((prev: T) => T)) => void] {
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

  // Re-read the stored value when another window announces a change.
  useEffect(() => {
    if (!syncEvent || typeof window === 'undefined') {
      return;
    }
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void listen(syncEvent, () => {
      try {
        const raw = window.localStorage.getItem(key);
        if (raw == null) {
          return;
        }
        const parsed = read(raw);
        if (parsed != null) {
          setState(parsed);
        }
      } catch (error) {
        console.warn(readErrorMessage, error);
      }
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        // No Tauri IPC (unit tests, plain browser): nothing to subscribe to.
        console.warn('Unable to subscribe to stored-state sync event', error);
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [key, read, readErrorMessage, syncEvent]);

  const setStoredState = useCallback(
    (next: T | ((prev: T) => T)) => {
      setState((prev) => {
        const resolved = typeof next === 'function' ? (next as (prev: T) => T)(prev) : next;
        const normalized = normalize ? normalize(resolved) : resolved;

        if (typeof window !== 'undefined') {
          try {
            window.localStorage.setItem(key, write(normalized));
          } catch (error) {
            console.warn(writeErrorMessage, error);
          }
        }

        return normalized;
      });
    },
    [key, normalize, write, writeErrorMessage],
  );

  return [state, setStoredState];
}
