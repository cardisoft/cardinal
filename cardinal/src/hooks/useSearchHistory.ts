import { useRef, useCallback } from 'react';

type HistoryDirection = 'older' | 'newer';

type UseSearchHistoryOptions = {
  maxEntries?: number;
};

type UseSearchHistoryResult = {
  handleInputChange: (nextValue: string) => void;
  navigate: (direction: HistoryDirection) => string | null;
  ensureTailValue: (value: string) => void;
  resetCursorToTail: () => void;
  getCurrentValue: () => string;
};

const DEFAULT_MAX_HISTORY = 50;

export function useSearchHistory(
  options: UseSearchHistoryOptions = {},
): UseSearchHistoryResult {
  const maxEntries = options.maxEntries ?? DEFAULT_MAX_HISTORY;
  const historyRef = useRef<string[]>(['']);
  const cursorRef = useRef(0);

  const getTailIndex = useCallback(() => {
    const history = historyRef.current;
    return history.length > 0 ? history.length - 1 : 0;
  }, []);

  const clampHistory = useCallback(() => {
    const history = historyRef.current;
    if (history.length <= maxEntries) {
      return;
    }

    const overflow = history.length - maxEntries;
    history.splice(0, overflow);
    cursorRef.current = Math.max(cursorRef.current - overflow, 0);
  }, [maxEntries]);

  const pushEntry = useCallback(
    (value: string) => {
      const history = historyRef.current;
      if (!history.length) {
        history.push('');
      }
      history.push(value);
      clampHistory();
      cursorRef.current = getTailIndex();
    },
    [clampHistory, getTailIndex],
  );

  const updateTail = useCallback(
    (value: string) => {
      const history = historyRef.current;
      if (!history.length) {
        history.push(value);
      } else {
        history[history.length - 1] = value;
      }
      cursorRef.current = getTailIndex();
    },
    [getTailIndex],
  );

  const handleInputChange = useCallback(
    (nextValue: string) => {
      const history = historyRef.current;
      if (!history.length) {
        history.push('');
      }
      const tailIndex = history.length - 1;
      const isAtTail = cursorRef.current === tailIndex;
      const tailValue = history[tailIndex] ?? '';

      if (!isAtTail) {
        pushEntry(nextValue);
        return;
      }

      if (tailValue !== '' && nextValue === '') {
        pushEntry(nextValue);
        return;
      }

      const firstLetterChanged =
        tailValue.length > 0 &&
        nextValue.length > 0 &&
        tailValue[0] !== nextValue[0];
      if (firstLetterChanged) {
        pushEntry(nextValue);
        return;
      }

      updateTail(nextValue);
    },
    [pushEntry, updateTail],
  );

  const navigate = useCallback((direction: HistoryDirection) => {
    const history = historyRef.current;
    if (!history.length) {
      return null;
    }
    const tailIndex = history.length - 1;
    let nextCursor = cursorRef.current;
    if (direction === 'older') {
      if (nextCursor === 0) {
        return null;
      }
      nextCursor -= 1;
    } else {
      if (nextCursor >= tailIndex) {
        return null;
      }
      nextCursor += 1;
    }
    cursorRef.current = nextCursor;
    return history[nextCursor] ?? '';
  }, []);

  const ensureTailValue = useCallback(
    (value: string) => {
      const history = historyRef.current;
      if (!history.length) {
        history.push(value);
        cursorRef.current = history.length - 1;
        return;
      }
      const tailIndex = history.length - 1;
      if (history[tailIndex] !== value) {
        pushEntry(value);
      } else {
        cursorRef.current = tailIndex;
      }
    },
    [pushEntry],
  );

  const resetCursorToTail = useCallback(() => {
    const history = historyRef.current;
    if (!history.length) {
      history.push('');
    }
    cursorRef.current = history.length - 1;
  }, []);

  const getCurrentValue = useCallback(() => {
    const history = historyRef.current;
    if (!history.length) {
      return '';
    }
    const cursor = Math.min(cursorRef.current, history.length - 1);
    return history[cursor] ?? '';
  }, []);

  return {
    handleInputChange,
    navigate,
    ensureTailValue,
    resetCursorToTail,
    getCurrentValue,
  };
}
