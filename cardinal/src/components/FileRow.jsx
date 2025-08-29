import React from 'react';
import { MiddleEllipsisHighlight, splitTextWithHighlight } from './MiddleEllipsisHighlight';
import { formatKB } from '../utils/format';

export function FileRow({ item, rowIndex, style, onContextMenu, searchQuery }) {
  if (!item) {
    // 显示加载状态而不是空白
    return null
  }

  const path = typeof item === 'string' ? item : item?.path;
  let filename = '', directoryPath = '';
  if (path) {
    const parts = path.split(/[\\/]/);
    filename = parts.pop() || '';
    directoryPath = parts.join('/');
  }

  const mtimeSec = typeof item !== 'string' ? (item?.metadata?.mtime ?? item?.mtime) : undefined;
  const mtimeText = mtimeSec != null ? new Date(mtimeSec * 1000).toLocaleString() : null;

  const ctimeSec = typeof item !== 'string' ? (item?.metadata?.ctime ?? item?.ctime) : undefined;
  const ctimeText = ctimeSec != null ? new Date(ctimeSec * 1000).toLocaleString() : null;

  const sizeBytes = typeof item !== 'string' ? (item?.metadata?.size ?? item?.size) : undefined;
  const sizeText = formatKB(sizeBytes);

  const handleContextMenu = (e) => {
    e.preventDefault();
    if (path && onContextMenu) {
      onContextMenu(e, path);
    }
  };

  return (
    <div style={style} className={`row ${rowIndex % 2 === 0 ? 'row-even' : 'row-odd'}`} onContextMenu={handleContextMenu}>
      <div className="columns row-inner" title={path}>
        <MiddleEllipsisHighlight className="filename-text" text={filename} searchQuery={searchQuery} />
        {/* Path 列显示目录路径（不包含文件名） */}
        <span className="path-text" title={directoryPath}>
          {splitTextWithHighlight(directoryPath || '', searchQuery).map((part, i) => (
            part.isHighlight ? <strong key={i}>{part.text}</strong> : <span key={i}>{part.text}</span>
          ))}
        </span>
        <span className={`size-text ${!sizeText ? 'muted' : ''}`}>{sizeText || '—'}</span>
        <span className={`mtime-text ${!mtimeText ? 'muted' : ''}`}>{mtimeText || '—'}</span>
        <span className={`ctime-text ${!ctimeText ? 'muted' : ''}`}>{ctimeText || '—'}</span>
      </div>
    </div>
  );
}
