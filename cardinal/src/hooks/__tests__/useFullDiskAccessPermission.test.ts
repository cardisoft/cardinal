import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  checkFullDiskAccessPermission,
  requestFullDiskAccessPermission,
} from 'tauri-plugin-macos-permissions-api';
import { FDA_POLL_INTERVAL_MS, useFullDiskAccessPermission } from '../useFullDiskAccessPermission';

vi.mock('tauri-plugin-macos-permissions-api', () => ({
  checkFullDiskAccessPermission: vi.fn(),
  requestFullDiskAccessPermission: vi.fn(),
}));

const mockedCheck = vi.mocked(checkFullDiskAccessPermission);
const mockedRequest = vi.mocked(requestFullDiskAccessPermission);

describe('useFullDiskAccessPermission', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('reports granted when the probe succeeds', async () => {
    mockedCheck.mockResolvedValue(true);
    const { result } = renderHook(() => useFullDiskAccessPermission());

    await waitFor(() => expect(result.current.isChecking).toBe(false));
    expect(result.current.status).toBe('granted');
  });

  it('picks up a grant made in System Settings without restarting the app', async () => {
    // Regression: the status used to be read once per launch, so granting access while the
    // app was running left it stuck on the permission overlay until a restart.
    mockedCheck.mockResolvedValue(false);
    const { result } = renderHook(() => useFullDiskAccessPermission());

    await waitFor(() => expect(result.current.status).toBe('denied'));

    mockedCheck.mockResolvedValue(true);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(FDA_POLL_INTERVAL_MS + 50);
    });

    await waitFor(() => expect(result.current.status).toBe('granted'));
  });

  it('polls only while denied', async () => {
    mockedCheck.mockResolvedValue(true);
    renderHook(() => useFullDiskAccessPermission());

    await waitFor(() => expect(mockedCheck).toHaveBeenCalledTimes(1));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(FDA_POLL_INTERVAL_MS * 3);
    });

    // An authorised app must not keep hitting the filesystem forever.
    expect(mockedCheck).toHaveBeenCalledTimes(1);
  });

  it('re-checks immediately when the window regains focus', async () => {
    mockedCheck.mockResolvedValue(false);
    const { result } = renderHook(() => useFullDiskAccessPermission());
    await waitFor(() => expect(result.current.status).toBe('denied'));

    const callsBefore = mockedCheck.mock.calls.length;
    mockedCheck.mockResolvedValue(true);

    act(() => {
      window.dispatchEvent(new Event('focus'));
    });

    await waitFor(() => expect(result.current.status).toBe('granted'));
    expect(mockedCheck.mock.calls.length).toBeGreaterThan(callsBefore);
  });

  it('exposes a manual recheck that ignores the poll interval', async () => {
    mockedCheck.mockResolvedValue(false);
    const { result } = renderHook(() => useFullDiskAccessPermission());
    await waitFor(() => expect(result.current.status).toBe('denied'));

    mockedCheck.mockResolvedValue(true);
    await act(async () => {
      await result.current.recheck();
    });

    expect(result.current.status).toBe('granted');
  });

  it('opens system settings and re-probes right after', async () => {
    mockedCheck.mockResolvedValue(false);
    mockedRequest.mockResolvedValue(undefined);
    const { result } = renderHook(() => useFullDiskAccessPermission());
    await waitFor(() => expect(result.current.status).toBe('denied'));

    const callsBefore = mockedCheck.mock.calls.length;
    await act(async () => {
      await result.current.requestPermission();
    });

    expect(mockedRequest).toHaveBeenCalled();
    expect(mockedCheck.mock.calls.length).toBeGreaterThan(callsBefore);
  });

  it('keeps the previous status when the probe times out', async () => {
    mockedCheck.mockResolvedValue(true);
    const { result } = renderHook(() => useFullDiskAccessPermission());
    await waitFor(() => expect(result.current.status).toBe('granted'));

    // A hung probe must not downgrade an authorised app (that would pop the overlay).
    mockedCheck.mockReturnValue(new Promise<boolean>(() => {}));
    await act(async () => {
      await result.current.recheck();
    });

    expect(result.current.status).toBe('granted');
  });
});
