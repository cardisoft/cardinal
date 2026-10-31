import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useBookmarkOpenListener } from '../useBookmarkOpenListener';

const mocks = vi.hoisted(() => ({
  listen: vi.fn(),
  handlers: new Map<string, (event: { payload: unknown }) => void>(),
  unlistened: 0,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: (event: { payload: unknown }) => void) => {
    mocks.listen(event, handler);
    mocks.handlers.set(event, handler);
    // Matches the real API: resolves on a later microtask, not synchronously.
    return Promise.resolve(() => {
      mocks.unlistened += 1;
      mocks.handlers.delete(event);
    });
  },
}));

const event = 'cardinal:bookmark-open';

const payload = {
  bookmark: {
    id: 'b1',
    categoryId: 'default',
    name: '天川空 极品',
    query: '天川空',
    filterSearch: '',
  },
  mode: 'new' as const,
};

const subscriptions = () => mocks.listen.mock.calls.filter(([name]) => name === event).length;

describe('useBookmarkOpenListener', () => {
  beforeEach(() => {
    mocks.listen.mockClear();
    mocks.handlers.clear();
    mocks.unlistened = 0;
  });

  it('subscribes once even when the callback changes identity on every render', async () => {
    // Regression: one "search in new tab" click opened N tabs, because the effect depended on
    // a callback that `useSearchTabs` rebuilt on every render. `listen()` resolves
    // asynchronously, so each re-run leaked a subscription instead of replacing it.
    const { rerender } = renderHook(
      ({ seq }: { seq: number }) => useBookmarkOpenListener(() => seq),
      { initialProps: { seq: 0 } },
    );

    await waitFor(() => expect(subscriptions()).toBe(1));

    for (let seq = 1; seq <= 10; seq += 1) {
      rerender({ seq });
      await act(async () => {
        await Promise.resolve();
      });
    }
    // Let any late unlisten callbacks settle.
    await act(async () => {
      await Promise.resolve();
    });

    expect(subscriptions()).toBe(1);
    expect(mocks.unlistened).toBe(0);
  });

  it('delivers each event exactly once', async () => {
    const onOpen = vi.fn();
    renderHook(({ seq }: { seq: number }) => useBookmarkOpenListener(() => onOpen(seq)), {
      initialProps: { seq: 0 },
    });
    await waitFor(() => expect(subscriptions()).toBe(1));

    const handler = mocks.handlers.get(event);
    expect(handler).toBeDefined();

    act(() => {
      handler?.({ payload });
      handler?.({ payload });
    });

    expect(onOpen).toHaveBeenCalledTimes(2);
  });

  it('calls the latest callback, not the one captured at subscription time', async () => {
    const calls: number[] = [];
    const { rerender } = renderHook(
      ({ seq }: { seq: number }) => useBookmarkOpenListener(() => calls.push(seq)),
      { initialProps: { seq: 1 } },
    );
    await waitFor(() => expect(subscriptions()).toBe(1));

    rerender({ seq: 2 });
    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      mocks.handlers.get(event)?.({ payload });
    });

    expect(calls).toEqual([2]);
  });

  it('unsubscribes on unmount without leaking when listen is still in flight', async () => {
    const { unmount } = renderHook(
      ({ seq }: { seq: number }) => useBookmarkOpenListener(() => seq),
      {
        initialProps: { seq: 0 },
      },
    );
    unmount();

    // Give the pending `listen()` promise a chance to resolve into the cancelled branch.
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(mocks.unlistened).toBe(1);
  });
});
