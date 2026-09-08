import { describe, expect, it } from 'vitest';
import { mergeFileOperation } from '../useFileOperationLog';

describe('mergeFileOperation', () => {
  it('keeps concurrent operations separate when they complete out of order', () => {
    const running = [
      { id: 'b', kind: 'trash' as const, state: 'running' as const, itemCount: 2 },
      { id: 'a', kind: 'trash' as const, state: 'running' as const, itemCount: 1 },
    ];
    const bCompleted = mergeFileOperation(running, {
      id: 'b',
      kind: 'trash',
      state: 'completed',
      itemCount: 2,
    });
    const aFailed = mergeFileOperation(bCompleted, {
      id: 'a',
      kind: 'trash',
      state: 'failed',
      itemCount: 1,
    });

    expect(aFailed).toEqual([
      expect.objectContaining({ id: 'a', state: 'failed' }),
      expect.objectContaining({ id: 'b', state: 'completed' }),
    ]);
  });
});
