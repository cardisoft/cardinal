import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { subscribeFSEventsBatch } from '../../runtime/tauriEventRuntime';
import { useRecentFSEvents } from '../useRecentFSEvents';

vi.mock('../../runtime/tauriEventRuntime', () => ({
  subscribeFSEventsBatch: vi.fn(),
}));

const mockedSubscribeFSEventsBatch = vi.mocked(subscribeFSEventsBatch);

describe('useRecentFSEvents', () => {
  const unlisten = vi.fn();
  let fsEventsBatchListener: ((payload: any) => void) | null = null;

  beforeEach(() => {
    vi.clearAllMocks();
    fsEventsBatchListener = null;

    mockedSubscribeFSEventsBatch.mockImplementation((callback: (payload: any) => void) => {
      fsEventsBatchListener = callback;
      return unlisten;
    });
  });

  it('stores and filters streamed events when active', async () => {
    const { result } = renderHook(() =>
      useRecentFSEvents({ caseSensitive: false, isActive: true }),
    );

    await waitFor(() => {
      expect(fsEventsBatchListener).not.toBeNull();
    });

    act(() => {
      fsEventsBatchListener?.([
        { path: '/tmp/Alpha.txt', eventId: 1, timestamp: 1, flagBits: 0 },
        { path: '/tmp/beta.txt', eventId: 2, timestamp: 2, flagBits: 0 },
      ]);
    });

    expect(result.current.filteredEvents).toHaveLength(2);

    act(() => {
      result.current.setEventFilterQuery('alpha');
    });

    expect(result.current.filteredEvents).toHaveLength(1);
    expect(result.current.filteredEvents[0]?.path).toBe('/tmp/Alpha.txt');
  });

  it('cleans up runtime subscription on unmount', async () => {
    const { unmount } = renderHook(() =>
      useRecentFSEvents({ caseSensitive: false, isActive: true }),
    );
    unmount();

    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});
