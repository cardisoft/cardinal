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
  getSelectedPaths?: () => string[],
): UseContextMenuResult {
  const { t } = useTranslation();

  const buildFileMenuItems = useCallback(
    (path: string): MenuItemOptions[] => {
      if (!path) {
        return [];
      }

      const selected = getSelectedPaths?.().filter(Boolean);
      const targetPaths = selected && selected.length > 0 ? selected : [path];
      const copyLabel =
        targetPaths.length > 1 ? t('contextMenu.copyFiles') : t('contextMenu.copyFile');
      const copyFilenameLabel =
        targetPaths.length > 1 ? t('contextMenu.copyFilenames') : t('contextMenu.copyFilename');
      const copyPathLabel =
        targetPaths.length > 1 ? t('contextMenu.copyPaths') : t('contextMenu.copyPath');
      const items: MenuItemOptions[] = [
        {
          id: 'context_menu.open_item',
          text: t('contextMenu.openItem'),
          accelerator: 'Cmd+O',
          action: () => {
            targetPaths.forEach((itemPath) => openResultPath(itemPath));
          },
        },
        {
          id: 'context_menu.open_in_finder',
          text: t('contextMenu.revealInFinder'),
          accelerator: 'Cmd+R',
          action: () => {
            targetPaths.forEach((itemPath) => {
              void invoke('open_in_finder', { path: itemPath });
            });
          },
        },
        {
          id: 'context_menu.copy_filename',
          text: copyFilenameLabel,
          action: () => {
            if (navigator?.clipboard?.writeText) {
              const filenames = targetPaths
                .map((itemPath) => {
                  const segments = itemPath.split(/[\\/]/).filter(Boolean);
                  return segments.length > 0 ? segments[segments.length - 1] : itemPath;
                })
                .join(' ');
              void navigator.clipboard.writeText(filenames);
            }
          },
        },
        {
          id: 'context_menu.copy_paths',
          text: copyPathLabel,
          accelerator: 'Cmd+Shift+C',
          action: () => {
            if (navigator?.clipboard?.writeText) {
              void navigator.clipboard.writeText(targetPaths.join('\n'));
            }
          },
        },
        {
          id: 'context_menu.copy_files',
          text: copyLabel,
          accelerator: 'Cmd+C',
          action: () => {
            void invoke('copy_files_to_clipboard', { paths: targetPaths }).catch((error) => {
              console.error('Failed to copy files to clipboard', error);
            });
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
    [getSelectedPaths, onQuickLookRequest, t],
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
