import { useCallback } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Menu } from '@tauri-apps/api/menu';
import type { MenuItemOptions } from '@tauri-apps/api/menu';
import { useTranslation } from 'react-i18next';

type UseContextMenuResult = {
  showContextMenu: (event: ReactMouseEvent<HTMLElement>, path: string) => void;
  showHeaderContextMenu: (event: ReactMouseEvent<HTMLElement>) => void;
};

export function useContextMenu(autoFitColumns: (() => void) | null = null): UseContextMenuResult {
  const { t } = useTranslation();

  const buildFileMenuItems = useCallback(
    (path: string): MenuItemOptions[] => {
      if (!path) {
        return [];
      }

      const segments = path.split(/[\\/]/).filter(Boolean);
      const filename = segments.length > 0 ? segments[segments.length - 1] : path;

      return [
        {
          id: 'context_menu.open_in_finder',
          text: t('contextMenu.openInFinder'),
          action: () => {
            void invoke('open_in_finder', { path });
          },
        },
        {
          id: 'context_menu.copy_path',
          text: t('contextMenu.copyPath'),
          action: () => {
            if (navigator?.clipboard?.writeText) {
              void navigator.clipboard.writeText(path);
            }
          },
        },
        {
          id: 'context_menu.copy_filename',
          text: t('contextMenu.copyFilename'),
          action: () => {
            if (navigator?.clipboard?.writeText) {
              void navigator.clipboard.writeText(filename);
            }
          },
        },
      ];
    },
    [t],
  );

  const buildHeaderMenuItems = useCallback((): MenuItemOptions[] => {
    if (!autoFitColumns) {
      return [];
    }

    return [
      {
        id: 'context_menu.reset_column_widths',
        text: t('contextMenu.resetColumnWidths'),
        action: () => {
          autoFitColumns();
        },
      },
    ];
  }, [autoFitColumns, t]);

  const showMenu = useCallback(async (items: MenuItemOptions[]) => {
    if (!items.length) {
      return;
    }

    try {
      const menu = await Menu.new({ items });
      await menu.popup();
    } catch (error) {
      console.error('Failed to show context menu', error);
    }
  }, []);

  const showContextMenu = useCallback(
    (event: ReactMouseEvent<HTMLElement>, path: string) => {
      event.preventDefault();
      event.stopPropagation();
      void showMenu(buildFileMenuItems(path));
    },
    [buildFileMenuItems, showMenu],
  );

  const showHeaderContextMenu = useCallback(
    (event: ReactMouseEvent<HTMLElement>) => {
      event.preventDefault();
      event.stopPropagation();
      void showMenu(buildHeaderMenuItems());
    },
    [buildHeaderMenuItems, showMenu],
  );

  return {
    showContextMenu,
    showHeaderContextMenu,
  };
}
