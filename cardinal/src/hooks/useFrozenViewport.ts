import { useEffect, useLayoutEffect, useRef, useState } from 'react';

type FrozenViewport = {
  targetVersion: number;
  scrollTop: number;
  items: React.ReactNode[];
};

type UseFrozenViewportArgs = {
  dataVersion: number;
  scrollTop: number;
  renderedItems: React.ReactNode[];
  viewportReady: boolean;
};

// Keeps the last fully rendered viewport on screen during data-version swaps until the
// next viewport batch is ready.
export function useFrozenViewport({
  dataVersion,
  scrollTop,
  renderedItems,
  viewportReady,
}: UseFrozenViewportArgs): FrozenViewport | null {
  const [frozenViewport, setFrozenViewport] = useState<FrozenViewport | null>(null);
  const lastCommittedItemsRef = useRef<React.ReactNode[]>([]);
  const lastCommittedDataVersionRef = useRef(dataVersion);

  // Capture the previous viewport when the backend result-set changes so we can keep it
  // on screen until the next viewport batch has loaded.
  useLayoutEffect(() => {
    const previousDataVersion = lastCommittedDataVersionRef.current;
    if (previousDataVersion === dataVersion) {
      return;
    }

    lastCommittedDataVersionRef.current = dataVersion;
    const snapshot = lastCommittedItemsRef.current;
    if (snapshot.length === 0) {
      setFrozenViewport(null);
      return;
    }

    setFrozenViewport({
      targetVersion: dataVersion,
      scrollTop,
      items: snapshot,
    });
  }, [dataVersion, scrollTop]);

  // Keep the most recent fully rendered viewport around. On the next data-version swap we
  // render this snapshot in an overlay so the user keeps seeing the old screen until the new
  // viewport batch is ready.
  useLayoutEffect(() => {
    lastCommittedItemsRef.current = frozenViewport?.items ?? renderedItems;
  }, [frozenViewport, renderedItems]);

  // Swap from the frozen viewport to live rows only after the active window is ready.
  useEffect(() => {
    if (!frozenViewport) {
      return;
    }

    if (frozenViewport.targetVersion === dataVersion && viewportReady) {
      setFrozenViewport(null);
    }
  }, [dataVersion, frozenViewport, viewportReady]);

  // If the user scrolls while a frozen viewport is showing, drop it instead of stretching
  // old content over a different viewport.
  useEffect(() => {
    if (frozenViewport && frozenViewport.scrollTop !== scrollTop) {
      setFrozenViewport(null);
    }
  }, [frozenViewport, scrollTop]);

  return frozenViewport;
}
