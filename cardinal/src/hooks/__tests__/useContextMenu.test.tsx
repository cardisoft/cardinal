import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { I18nextProvider } from 'react-i18next';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n/config';
import { useContextMenu } from '../useContextMenu';

const popupMock = vi.fn().mockResolvedValue(undefined);
const menuNewMock = vi.fn().mockResolvedValue({ popup: popupMock });
const invokeMock = vi.fn().mockResolvedValue(undefined);

vi.mock('@tauri-apps/api/menu', () => ({
  Menu: { new: menuNewMock },
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('../../utils/openResultPath', () => ({
  openResultPath: vi.fn(),
}));

const wrapper = ({ children }: { children: ReactNode }) => (
  <I18nextProvider i18n={i18n}>{children}</I18nextProvider>
);

const createEvent = () =>
  ({
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  }) as unknown as React.MouseEvent<HTMLElement>;

describe('useContextMenu', () => {
  beforeEach(async () => {
    menuNewMock.mockClear();
    popupMock.mockClear();
    invokeMock.mockClear();
    await i18n.changeLanguage('en-US');
  });

  it('uses plural Copy Paths label and shortcut when multiple paths are selected', async () => {
    const getSelectedPaths = () => ['/a', '/b'];
    const { result } = renderHook(() => useContextMenu(null, undefined, getSelectedPaths), {
      wrapper,
    });

    result.current.showContextMenu(createEvent(), '/a');

    await waitFor(() => {
      expect(menuNewMock).toHaveBeenCalled();
    });

    const items = menuNewMock.mock.calls[0][0].items as Array<{
      id: string;
      text?: string;
      accelerator?: string;
    }>;
    const copyPaths = items.find((item) => item.id === 'context_menu.copy_paths');
    expect(copyPaths?.text).toBe('Copy Paths');
    expect(copyPaths?.accelerator).toBe('Cmd+Shift+C');
  });

  it('uses singular Copy Path label when a single path is targeted', async () => {
    const getSelectedPaths = () => [];
    const { result } = renderHook(() => useContextMenu(null, undefined, getSelectedPaths), {
      wrapper,
    });

    result.current.showContextMenu(createEvent(), '/a');

    await waitFor(() => {
      expect(menuNewMock).toHaveBeenCalled();
    });

    const items = menuNewMock.mock.calls[0][0].items as Array<{
      id: string;
      text?: string;
      accelerator?: string;
    }>;
    const copyPaths = items.find((item) => item.id === 'context_menu.copy_paths');
    expect(copyPaths?.text).toBe('Copy Path');
  });

  it('copies all selected paths to the clipboard', async () => {
    const getSelectedPaths = () => ['/a', '/b'];
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(globalThis.navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
    });

    const { result } = renderHook(() => useContextMenu(null, undefined, getSelectedPaths), {
      wrapper,
    });

    result.current.showContextMenu(createEvent(), '/a');

    await waitFor(() => {
      expect(menuNewMock).toHaveBeenCalled();
    });

    const items = menuNewMock.mock.calls[0][0].items as Array<{
      id: string;
      action?: () => void;
    }>;
    const copyPaths = items.find((item) => item.id === 'context_menu.copy_paths');
    copyPaths?.action?.();

    expect(writeText).toHaveBeenCalledWith('/a\n/b');
  });
});
