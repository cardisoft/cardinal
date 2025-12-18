import { useCallback } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Menu } from '@tauri-apps/api/menu';
import type { MenuItemOptions } from '@tauri-apps/api/menu';
import { useTranslation } from 'react-i18next';
import { openResultPath } from '../utils/openResultPath';

type UseContextMenuResult = {
  showContextMenu: (event: ReactMouseEvent<HTMLElement>, path: string) => void;
  showHeaderContextMenu: (event: ReactMouseEvent<HTMLElement>) => void;
};

export function useContextMenu(
  autoFitColumns: (() => void) | null = null,
  onQuickLookRequest?: () => void | Promise<void>,
  onTrashRequest?: (path: string) => void | Promise<void>,
  onDeleteRequest?: (path: string) => void | Promise<void>,
): UseContextMenuResult {
  const { t } = useTranslation();

  const buildFileMenuItems = useCallback(
    (path: string): MenuItemOptions[] => {
      if (!path) {
        return [];
      }

      const segments = path.split(/[\\/]/).filter(Boolean);
      const filename = segments.length > 0 ? segments[segments.length - 1] : path;

      const items: MenuItemOptions[] = [
        {
          id: 'context_menu.open_item',
          text: t('contextMenu.openItem'),
          accelerator: 'Cmd+O',
          action: () => {
            openResultPath(path);
          },
        },
        {
          id: 'context_menu.open_in_finder',
          text: t('contextMenu.revealInFinder'),
          accelerator: 'Cmd+R',
          action: () => {
            void invoke('open_in_finder', { path });
          },
        },
        {
          id: 'context_menu.copy_path',
          text: t('contextMenu.copyPath'),
          accelerator: 'Cmd+C',
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
        {
          id: 'context_menu.trash',
          text: t('contextMenu.moveToTrash'),
          accelerator: 'Cmd+Backspace',
          action: () => {
            if (onTrashRequest) {
              void onTrashRequest(path);
            }
          },
        },
        {
          id: 'context_menu.delete_permanent',
          text: t('contextMenu.deletePermanently'),
          accelerator: 'Opt+Cmd+Backspace',
          action: () => {
            if (onDeleteRequest) {
              void onDeleteRequest(path);
            }
          },
        },
      ];

      if (onQuickLookRequest) {
        items.push({
          id: 'context_menu.quicklook',
          text: t('contextMenu.quickLook'),
          accelerator: 'Space',
          action: () => {
            if (onQuickLookRequest) {
              void onQuickLookRequest();
            }
          },
        });
      }

      return items;
    },
    [onQuickLookRequest, onTrashRequest, onDeleteRequest, t],
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
