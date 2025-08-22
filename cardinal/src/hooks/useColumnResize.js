import { useState, useRef, useCallback } from 'react';
import { calculateInitialColWidths, MAX_COL_WIDTH, MIN_COL_WIDTH } from '../constants';

export function useColumnResize() {
  const [colWidths, setColWidths] = useState(() => {
    // 初始化时根据窗口宽度计算列宽
    const windowWidth = window.innerWidth;
    return calculateInitialColWidths(windowWidth);
  });
  const resizingRef = useRef(null);

  const onResizeStart = useCallback((key) => (e) => {
    e.preventDefault();
    e.stopPropagation();
    
    resizingRef.current = { 
      key, 
      startX: e.clientX, 
      startW: colWidths[key] 
    };
    
    window.addEventListener('mousemove', onResizing);
    window.addEventListener('mouseup', onResizeEnd, { once: true });
    
    document.body.style.userSelect = 'none';
    document.body.style.cursor = 'col-resize';
  }, [colWidths]);

  const autoFitColumns = useCallback(() => {
    const windowWidth = window.innerWidth;
    const newColWidths = calculateInitialColWidths(windowWidth);
    setColWidths(newColWidths);
  }, []);

  const onResizing = useCallback((e) => {
    const ctx = resizingRef.current;
    if (!ctx) return;
    
    const delta = e.clientX - ctx.startX;
    const minW = MIN_COL_WIDTH; // 最小宽度限制
    const maxW = MAX_COL_WIDTH; // 最大宽度限制
    const nextW = Math.max(minW, Math.min(maxW, ctx.startW + delta));
    
    setColWidths((w) => ({ ...w, [ctx.key]: nextW }));
  }, []);

  const onResizeEnd = useCallback(() => {
    resizingRef.current = null;
    window.removeEventListener('mousemove', onResizing);
    document.body.style.userSelect = '';
    document.body.style.cursor = '';
  }, [onResizing]);

  return {
    colWidths,
    onResizeStart,
    autoFitColumns
  };
}
