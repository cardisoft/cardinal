import React, { useRef, useCallback, useEffect } from 'react';
import { SCROLLBAR_THUMB_MIN } from '../constants';

export const Scrollbar = ({
  totalHeight,
  viewportHeight,
  maxScrollTop,
  scrollTop,
  onScrollUpdate,
}) => {
  const scrollTrackRef = useRef(null);
  const scrollThumbRef = useRef(null);
  const isDraggingRef = useRef(false);

  // 更新滚动条位置和大小
  const updateScrollbar = useCallback(() => {
    const track = scrollTrackRef.current;
    const thumb = scrollThumbRef.current;
    if (!track || !thumb || totalHeight <= viewportHeight) {
      if (thumb) thumb.style.display = 'none';
      return;
    }
    thumb.style.display = 'block';
    const trackHeight = track.clientHeight;
    const thumbHeight = Math.max(SCROLLBAR_THUMB_MIN, (viewportHeight / totalHeight) * trackHeight);
    const thumbTop =
      maxScrollTop > 0 ? (scrollTop / maxScrollTop) * (trackHeight - thumbHeight) : 0;
    thumb.style.height = `${thumbHeight}px`;
    thumb.style.transform = `translateY(${thumbTop}px)`;
  }, [totalHeight, viewportHeight, maxScrollTop, scrollTop]);

  // 滚动条拖拽处理
  const handleThumbMouseDown = useCallback(
    (e) => {
      e.preventDefault();
      isDraggingRef.current = true;
      const track = scrollTrackRef.current;
      const thumb = scrollThumbRef.current;
      if (!track || !thumb) return;
      track.classList.add('is-dragging');
      const trackRect = track.getBoundingClientRect();
      const thumbRect = thumb.getBoundingClientRect();
      const trackHeight = trackRect.height;
      const thumbHeight = thumbRect.height;
      const mouseOffsetInThumb = e.clientY - thumbRect.top;
      const handleMouseMove = (moveEvent) => {
        if (!isDraggingRef.current) return;
        const mousePositionInTrack = moveEvent.clientY - trackRect.top - mouseOffsetInThumb;
        const maxThumbTop = trackHeight - thumbHeight;
        const clampedThumbTop = Math.max(0, Math.min(mousePositionInTrack, maxThumbTop));
        const scrollRatio = maxThumbTop > 0 ? clampedThumbTop / maxThumbTop : 0;
        const newScrollTop = scrollRatio * maxScrollTop;
        onScrollUpdate(() => newScrollTop);
      };
      const handleMouseUp = () => {
        isDraggingRef.current = false;
        track.classList.remove('is-dragging');
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    },
    [maxScrollTop, onScrollUpdate],
  );

  // 滚动条轨道点击
  const handleTrackClick = useCallback(
    (e) => {
      if (e.target === scrollThumbRef.current) return;
      const rect = scrollTrackRef.current?.getBoundingClientRect();
      if (!rect) return;
      const clickY = e.clientY - rect.top;
      const scrollRatio = clickY / rect.height;
      const newScrollTop = scrollRatio * maxScrollTop;
      onScrollUpdate(() => newScrollTop);
    },
    [maxScrollTop, onScrollUpdate],
  );

  // 当 props 变化时更新滚动条
  useEffect(() => {
    updateScrollbar();
  }, [updateScrollbar]);

  return (
    <div className="virtual-scrollbar">
      <div ref={scrollTrackRef} className="virtual-scrollbar-track" onClick={handleTrackClick}>
        <div
          ref={scrollThumbRef}
          className="virtual-scrollbar-thumb"
          onMouseDown={handleThumbMouseDown}
        />
      </div>
    </div>
  );
};

export default Scrollbar;
