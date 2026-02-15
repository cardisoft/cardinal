import type { MouseEvent as ReactMouseEvent } from 'react';

type StartColumnResizeDragOptions = {
  event: ReactMouseEvent<HTMLSpanElement>;
  startWidth: number;
  clampWidth: (value: number) => number;
  applyWidth: (nextWidth: number) => void;
};

export const startColumnResizeDrag = ({
  event,
  startWidth,
  clampWidth,
  applyWidth,
}: StartColumnResizeDragOptions): void => {
  event.preventDefault();
  event.stopPropagation();

  const resizerElement = event.currentTarget;
  const startX = event.clientX;
  resizerElement.classList.add('col-resizer--dragging');

  const handleMouseMove = (moveEvent: MouseEvent) => {
    moveEvent.preventDefault();
    document.body.style.cursor = 'col-resize';
    const delta = moveEvent.clientX - startX;
    applyWidth(clampWidth(startWidth + delta));
  };

  const handleMouseUp = () => {
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);
    document.body.style.userSelect = '';
    document.body.style.cursor = '';
    resizerElement.classList.remove('col-resizer--dragging');
  };

  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', handleMouseUp);
  document.body.style.userSelect = 'none';
  document.body.style.cursor = 'col-resize';
};
