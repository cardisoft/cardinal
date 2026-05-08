import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

import { isValidEndpoint, useServerConfig } from '../useServerConfig';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const STORAGE_KEY = 'cardinal.serverConfig';

const mockedInvoke = vi.mocked(invoke);

describe('useServerConfig', () => {
  beforeEach(() => {
    window.localStorage.clear();
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValue({
      enabled: false,
      endpoint: '127.0.0.1:3388',
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('hydrates from the backend and overwrites stale local storage', async () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        enabled: true,
        endpoint: '0.0.0.0:3390',
      }),
    );
    mockedInvoke.mockResolvedValue({
      enabled: false,
      endpoint: '127.0.0.1:3388',
    });

    const { result } = renderHook(() => useServerConfig());

    expect(result.current.serverConfig).toEqual({
      enabled: true,
      endpoint: '0.0.0.0:3390',
    });

    await waitFor(() => {
      expect(result.current.serverConfig).toEqual({
        enabled: false,
        endpoint: '127.0.0.1:3388',
      });
    });

    expect(mockedInvoke).toHaveBeenCalledWith('get_server_config');
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe(
      JSON.stringify({
        enabled: false,
        endpoint: '127.0.0.1:3388',
      }),
    );
  });

  it('persists normalized updates locally', async () => {
    const { result } = renderHook(() => useServerConfig());

    await waitFor(() => {
      expect(result.current.serverConfig).toEqual({
        enabled: false,
        endpoint: '127.0.0.1:3388',
      });
    });

    act(() => {
      result.current.setServerConfig({
        enabled: true,
        endpoint: ' 0.0.0.0:3390 ',
      });
    });

    expect(result.current.serverConfig).toEqual({
      enabled: true,
      endpoint: '0.0.0.0:3390',
    });
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe(
      JSON.stringify({
        enabled: true,
        endpoint: '0.0.0.0:3390',
      }),
    );
  });

  it('rejects ports with non-digit suffixes', () => {
    expect(isValidEndpoint('127.0.0.1:3388abc')).toBe(false);
    expect(isValidEndpoint('127.0.0.1:33 88')).toBe(false);
    expect(isValidEndpoint('127.0.0.1:3388')).toBe(true);
  });
});
