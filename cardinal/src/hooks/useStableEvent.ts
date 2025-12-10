import { useCallback, useEffect, useRef } from 'react';

/**
 * Returns a stable callback whose body always sees the latest handler.
 * Avoids having to re-register global listeners when dependencies change.
 *
 * Replace this with useEffectEvent after React 19:
 */
export function useStableEvent<T extends (...args: any[]) => any>(handler: T): T {
  const handlerRef = useRef(handler);

  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useCallback(
    ((...args: Parameters<T>) => {
      return handlerRef.current?.(...args);
    }) as T,
    [],
  );
}
