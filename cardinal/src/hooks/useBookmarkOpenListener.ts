import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { BOOKMARK_OPEN_EVENT, type BookmarkOpenPayload } from '../constants/appEvents';
import { useStableEvent } from './useStableEvent';

/**
 * Subscribes the main window to bookmark-open requests coming from the bookmarks window.
 *
 * The handler is wrapped in `useStableEvent`, so this effect subscribes exactly once even though
 * the caller's callback is rebuilt on every render. That matters: `listen()` resolves
 * asynchronously, so an effect that re-runs on every render cannot clean up its previous
 * subscription (`unlisten` is still `undefined`), and every render leaks a listener. One click
 * in the bookmarks window then opened as many tabs as there had been renders.
 */
export function useBookmarkOpenListener(onOpen: (payload: BookmarkOpenPayload) => void): void {
  const stableOpen = useStableEvent(onOpen);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void listen<BookmarkOpenPayload>(BOOKMARK_OPEN_EVENT, (event) => {
      stableOpen(event.payload);
    })
      .then((fn) => {
        // The component may have unmounted while `listen()` was in flight.
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        // No Tauri IPC (unit tests, plain browser): nothing to subscribe to.
        console.error('Failed to register bookmark-open listener', error);
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [stableOpen]);
}
