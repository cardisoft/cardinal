import { useCallback } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Menu } from '@tauri-apps/api/menu';
import type { MenuItemOptions } from '@tauri-apps/api/menu';
import { useTranslation } from 'react-i18next';
import type { ShortcutId, ShortcutMap } from '../shortcuts';
import { openResultPath } from '../utils/openResultPath';
import { splitPath } from '../utils/path';
import { formatShortcutForDisplay } from '../utils/shortcutCapture';

type UseContextMenuResult = {
  showContextMenu: (event: ReactMouseEvent<HTMLElement>, targetPaths: string[]) => void;
  showHeaderContextMenu: (event: ReactMouseEvent<HTMLElement>) => void;
};

type FileMenuActionDefinition = {
  id: string;
  text: string;
  shortcutId: ShortcutId;
  action: () => void;
};

export function useContextMenu(
  autoFitColumns: (() => void) | null = null,
  onQuickLookRequest: (() => void | Promise<void>) | undefined,
  shortcuts: ShortcutMap,
): UseContextMenuResult {
  const { t } = useTranslation();
  const writeClipboard = useCallback((text: string) => {
    if (!navigator?.clipboard?.writeText) {
      return;
    }
    void navigator.clipboard.writeText(text);
  }, []);

  const buildFileMenuItems = useCallback(
    (targetPathsInput: string[]): MenuItemOptions[] => {
      const targetPaths = targetPathsInput.filter(Boolean);
      if (targetPaths.length === 0) {
        return [];
      }

      const copyLabel =
        targetPaths.length > 1 ? t('contextMenu.copyFiles') : t('contextMenu.copyFile');
      const copyFilenameLabel =
        targetPaths.length > 1 ? t('contextMenu.copyFilenames') : t('contextMenu.copyFilename');
      const copyPathLabel =
        targetPaths.length > 1 ? t('contextMenu.copyPaths') : t('contextMenu.copyPath');

      const definitions: FileMenuActionDefinition[] = [
        {
          id: 'context_menu.open_item',
          text: t('contextMenu.openItem'),
          shortcutId: 'openResult',
          action: () => {
            targetPaths.forEach((itemPath) => openResultPath(itemPath));
          },
        },
        {
          id: 'context_menu.open_in_finder',
          text: t('contextMenu.revealInFinder'),
          shortcutId: 'revealInFinder',
          action: () => {
            targetPaths.forEach((itemPath) => {
              void invoke('open_in_finder', { path: itemPath });
            });
          },
        },
        {
          id: 'context_menu.copy_filename',
          text: copyFilenameLabel,
          shortcutId: 'copyFilenames',
          action: () => {
            const filenames = targetPaths
              .map((itemPath) => splitPath(itemPath).name || itemPath)
              .join(' ');
            writeClipboard(filenames);
          },
        },
        {
          id: 'context_menu.copy_paths',
          text: copyPathLabel,
          shortcutId: 'copyPaths',
          action: () => {
            writeClipboard(targetPaths.join('\n'));
          },
        },
        {
          id: 'context_menu.copy_files',
          text: copyLabel,
          shortcutId: 'copyFiles',
          action: () => {
            void invoke('copy_files_to_clipboard', { paths: targetPaths }).catch((error) => {
              console.error('Failed to copy files to clipboard', error);
            });
          },
        },
      ];

      if (onQuickLookRequest) {
        definitions.push({
          id: 'context_menu.quicklook',
          text: t('contextMenu.quickLook'),
          shortcutId: 'quickLook',
          action: () => {
            void onQuickLookRequest();
          },
        });
      }

      return definitions.map((definition) => ({
        id: definition.id,
        text: definition.text,
        accelerator: formatShortcutForDisplay(shortcuts[definition.shortcutId]),
        action: definition.action,
      }));
    },
    [onQuickLookRequest, shortcuts, t, writeClipboard],
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
    (event: ReactMouseEvent<HTMLElement>, targetPaths: string[]) => {
      event.preventDefault();
      event.stopPropagation();
      void showMenu(buildFileMenuItems(targetPaths));
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
