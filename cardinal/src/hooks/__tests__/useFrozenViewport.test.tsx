import React from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useFrozenViewport } from '../useFrozenViewport';

type HookProps = {
  dataVersion: number;
  scrollTop: number;
  renderedItems: React.ReactNode[];
  viewportReady: boolean;
};

const renderFrozenViewport = (initialProps: HookProps) =>
  renderHook(
    ({ dataVersion, scrollTop, renderedItems, viewportReady }: HookProps) =>
      useFrozenViewport({ dataVersion, scrollTop, renderedItems, viewportReady }),
    { initialProps },
  );

describe('useFrozenViewport', () => {
  it('captures the previous rendered viewport when data version changes', async () => {
    const firstItems = [<div key="old-a">old-a</div>, <div key="old-b">old-b</div>];
    const nextItems = [<div key="new-a">new-a</div>, <div key="new-b">new-b</div>];

    const { result, rerender } = renderFrozenViewport({
      dataVersion: 1,
      scrollTop: 0,
      renderedItems: firstItems,
      viewportReady: true,
    });

    act(() => {
      rerender({
        dataVersion: 2,
        scrollTop: 0,
        renderedItems: nextItems,
        viewportReady: false,
      });
    });

    await waitFor(() => {
      expect(result.current).not.toBeNull();
      expect(result.current?.targetVersion).toBe(2);
      expect(result.current?.items).toBe(firstItems);
    });
  });

  it('clears the frozen viewport after the next viewport reports ready', async () => {
    const firstItems = [<div key="old-a">old-a</div>, <div key="old-b">old-b</div>];
    const nextItems = [<div key="new-a">new-a</div>, <div key="new-b">new-b</div>];

    const { result, rerender } = renderFrozenViewport({
      dataVersion: 1,
      scrollTop: 0,
      renderedItems: firstItems,
      viewportReady: true,
    });

    act(() => {
      rerender({
        dataVersion: 2,
        scrollTop: 0,
        renderedItems: nextItems,
        viewportReady: false,
      });
    });

    await waitFor(() => {
      expect(result.current?.items).toBe(firstItems);
    });

    act(() => {
      rerender({
        dataVersion: 2,
        scrollTop: 0,
        renderedItems: nextItems,
        viewportReady: true,
      });
    });

    await waitFor(() => {
      expect(result.current).toBeNull();
    });
  });

  it('drops the frozen viewport if scroll position changes during the handoff', async () => {
    const firstItems = [<div key="old-a">old-a</div>, <div key="old-b">old-b</div>];
    const nextItems = [<div key="new-a">new-a</div>, <div key="new-b">new-b</div>];

    const { result, rerender } = renderFrozenViewport({
      dataVersion: 1,
      scrollTop: 0,
      renderedItems: firstItems,
      viewportReady: true,
    });

    act(() => {
      rerender({
        dataVersion: 2,
        scrollTop: 0,
        renderedItems: nextItems,
        viewportReady: false,
      });
    });

    await waitFor(() => {
      expect(result.current?.items).toBe(firstItems);
    });

    act(() => {
      rerender({
        dataVersion: 2,
        scrollTop: 20,
        renderedItems: nextItems,
        viewportReady: false,
      });
    });

    await waitFor(() => {
      expect(result.current).toBeNull();
    });
  });
});
