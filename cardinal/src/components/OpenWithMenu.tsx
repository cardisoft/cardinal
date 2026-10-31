import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type OpenWithApp = {
  bundleId: string;
  name: string;
  path: string;
  isDefault: boolean;
};

type OpenWithMenuProps = {
  /** File whose handlers should be listed. Multi-selection is intentionally not supported. */
  path: string;
  /** Viewport coordinates where the submenu should open. */
  x: number;
  y: number;
  labels: {
    title: string;
    loading: string;
    empty: string;
    chooseOther: string;
    defaultSuffix: string;
  };
  onClose: () => void;
};

const MENU_WIDTH = 240;

/**
 * "Open with" submenu, rendered as a child of the native context menu.
 *
 * Tauri's native menu has no submenu support (the sort menu hit the same limitation), so the
 * native menu only offers "Open With…"; this popup lists the applications LaunchServices knows
 * how to use for the file, mirroring Finder's own submenu.
 */
export function OpenWithMenu({
  path,
  x,
  y,
  labels,
  onClose,
}: OpenWithMenuProps): React.JSX.Element {
  const [apps, setApps] = useState<OpenWithApp[] | null>(null);
  const [position, setPosition] = useState({ x, y });
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    void invoke<{ apps: OpenWithApp[] } | null>('open_with_options', { path })
      .then((result) => {
        if (!cancelled) {
          setApps(result?.apps ?? []);
        }
      })
      .catch((error) => {
        console.error('Failed to list applications for', path, error);
        if (!cancelled) {
          setApps([]);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [path]);

  // Keep the popup inside the window, like the sort menu does.
  useEffect(() => {
    const height = menuRef.current?.offsetHeight ?? 0;
    setPosition({
      x: Math.max(8, Math.min(x, window.innerWidth - MENU_WIDTH - 8)),
      y: Math.max(8, Math.min(y, window.innerHeight - height - 8)),
    });
  }, [x, y, apps]);

  useEffect(() => {
    const handlePointerDown = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('pointerdown', handlePointerDown, true);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown, true);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [onClose]);

  const openWith = useCallback(
    (app: OpenWithApp) => {
      void invoke('open_with_app', {
        path,
        bundleId: app.bundleId,
        appPath: app.path,
      }).catch((error) => {
        console.error('Failed to open with application', app, error);
      });
      onClose();
    },
    [onClose, path],
  );

  return (
    <div
      ref={menuRef}
      className="open-with-menu"
      style={{ left: position.x, top: position.y }}
      role="menu"
      aria-label={labels.title}
      onContextMenu={(event) => event.preventDefault()}
    >
      {apps === null ? (
        <div className="open-with-menu__status">{labels.loading}</div>
      ) : apps.length === 0 ? (
        <div className="open-with-menu__status">{labels.empty}</div>
      ) : (
        apps.map((app) => (
          <button
            key={app.bundleId}
            type="button"
            role="menuitem"
            className="open-with-menu__item"
            onClick={() => openWith(app)}
          >
            <span className="open-with-menu__name">
              {app.name}
              {app.isDefault ? (
                <span className="open-with-menu__default"> ({labels.defaultSuffix})</span>
              ) : null}
            </span>
          </button>
        ))
      )}

      <div className="open-with-menu__separator" role="separator" />

      <button
        type="button"
        role="menuitem"
        className="open-with-menu__item"
        onClick={() => {
          // Finder's "Other…" panel: the OS owns the chooser, so reveal the file and let the
          // user pick a handler from the standard dialog.
          void invoke('choose_app_for_path', { path }).catch((error) => {
            console.error('Failed to open the application chooser', error);
          });
          onClose();
        }}
      >
        {labels.chooseOther}
      </button>
    </div>
  );
}

export default OpenWithMenu;
