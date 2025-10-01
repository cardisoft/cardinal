import React, { useMemo } from 'react';
import { MiddleEllipsisHighlight } from './MiddleEllipsisHighlight';
import { formatKB } from '../utils/format';

const SEGMENT_SEPARATOR = /[\\/]+/;

function deriveHighlightTerm(query) {
  if (!query) return '';
  const segments = query.split(SEGMENT_SEPARATOR).filter(Boolean);
  if (segments.length === 0) {
    return query.trim();
  }
  return segments[segments.length - 1].trim();
}

export function FileRow({ item, rowIndex, style, onContextMenu, searchQuery, caseInsensitive }) {
  const highlightTerm = useMemo(() => deriveHighlightTerm(searchQuery), [searchQuery]);
  if (!item || (typeof item !== 'string' && !item?.path)) {
    return null
  }

  const path = typeof item === 'string' ? item : item?.path;
  let filename = '', directoryPath = '';
  if (path) {
    if (path === '/') {
      directoryPath = '/';
    } else {
      const parts = path.split(/[\\/]/);
      filename = parts.pop() || '';
      directoryPath = parts.join('/');
    }
  }

  const mtimeSec = typeof item !== 'string' ? (item?.metadata?.mtime ?? item?.mtime) : undefined;
  const mtimeText = mtimeSec != null ? new Date(mtimeSec * 1000).toLocaleString() : null;

  const ctimeSec = typeof item !== 'string' ? (item?.metadata?.ctime ?? item?.ctime) : undefined;
  const ctimeText = ctimeSec != null ? new Date(ctimeSec * 1000).toLocaleString() : null;

  const sizeBytes = typeof item !== 'string' ? (item?.metadata?.size ?? item?.size) : undefined;
  const sizeText = item?.metadata?.type !== 1 ? formatKB(sizeBytes) : null;

  const handleContextMenu = (e) => {
    e.preventDefault();
    if (path && onContextMenu) {
      onContextMenu(e, path);
    }
  };

  return (
    <div
      style={style}
      className={`row columns ${rowIndex % 2 === 0 ? 'row-even' : 'row-odd'}`}
      onContextMenu={handleContextMenu}
      title={path}
    >
      <div className="filename-column">
        {item.icon ? (
          <img src={item.icon} alt="icon" className="file-icon" />
        ) : (
          <span className="file-icon file-icon-placeholder" aria-hidden="true" />
        )}
        <MiddleEllipsisHighlight
          className="filename-text"
          text={filename}
          highlightTerm={highlightTerm}
          caseInsensitive={caseInsensitive}
        />
      </div>
      {/* Path 列显示目录路径（不包含文件名） */}
      <span className="path-text" title={directoryPath}>
        {directoryPath}
      </span>
      <span className={`size-text ${!sizeText ? 'muted' : ''}`}>{sizeText || '—'}</span>
      <span className={`mtime-text ${!mtimeText ? 'muted' : ''}`}>{mtimeText || '—'}</span>
      <span className={`ctime-text ${!ctimeText ? 'muted' : ''}`}>{ctimeText || '—'}</span>
    </div>
  );
}
