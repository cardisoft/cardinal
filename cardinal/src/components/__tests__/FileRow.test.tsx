import React from 'react';
import { describe, expect, it, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import { FileRow } from '../FileRow';

const baseItem = {
  path: '/tmp/example.txt',
  metadata: { type: 0, size: 1024, mtime: 0, ctime: 0 },
};

const renderRow = (props?: Partial<React.ComponentProps<typeof FileRow>>) => {
  return render(
    <FileRow rowIndex={0} item={baseItem} isSelected={true} onSelect={vi.fn()} {...props} />,
  );
};

describe('FileRow selection interactions', () => {
  it('does not send redundant selection updates for a plain click on an already-selected row', () => {
    const onSelect = vi.fn();
    const { getByTitle } = renderRow({ onSelect });
    fireEvent.mouseDown(getByTitle(baseItem.path), { button: 0 });
    expect(onSelect).not.toHaveBeenCalled();
  });

  it('fires once on mouseup when a selected row is clicked without modifiers', () => {
    const onSelect = vi.fn();
    const { getByTitle } = renderRow({ onSelect });
    const node = getByTitle(baseItem.path);
    fireEvent.mouseDown(node, { button: 0 });
    expect(onSelect).not.toHaveBeenCalled();
    fireEvent.mouseUp(node, { button: 0 });
    expect(onSelect).toHaveBeenCalledWith(0, {
      isShift: false,
      isMeta: false,
      isCtrl: false,
    });
  });

  it('fires immediately when clicking an unselected row without modifiers', () => {
    const onSelect = vi.fn();
    const { getByTitle } = renderRow({ onSelect, isSelected: false });
    fireEvent.mouseDown(getByTitle(baseItem.path), { button: 0 });
    expect(onSelect).toHaveBeenCalledWith(0, {
      isShift: false,
      isMeta: false,
      isCtrl: false,
    });
  });

  it('tells the selection controller to extend the range when shift-clicking a selected row', () => {
    const onSelect = vi.fn();
    const { getByTitle } = renderRow({ onSelect });
    fireEvent.mouseDown(getByTitle(baseItem.path), { button: 0, shiftKey: true });
    expect(onSelect).toHaveBeenCalledWith(0, {
      isShift: true,
      isMeta: false,
      isCtrl: false,
    });
  });

  it('allows toggling selection via meta-click even if the row is already selected', () => {
    const onSelect = vi.fn();
    const { getByTitle } = renderRow({ onSelect });
    fireEvent.mouseDown(getByTitle(baseItem.path), { button: 0, metaKey: true });
    expect(onSelect).toHaveBeenCalledWith(0, {
      isShift: false,
      isMeta: true,
      isCtrl: false,
    });
  });

  it('does not fire when a pending click turns into a drag gesture', () => {
    const onSelect = vi.fn();
    const { getByTitle } = renderRow({ onSelect });
    const node = getByTitle(baseItem.path);
    fireEvent.mouseDown(node, { button: 0 });
    fireEvent.dragStart(node);
    expect(onSelect).not.toHaveBeenCalled();
  });
});
