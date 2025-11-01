import React, { useEffect, useRef, useState, useCallback, useMemo } from 'react';

const CHAR_WIDTH = 8; // Approximate monospace character width in pixels – used for quick truncation math.

export type HighlightSegment = {
  text: string;
  isHighlight: boolean;
};

type SplitOptions = {
  caseInsensitive?: boolean;
};

export function splitTextWithHighlight(
  text: string,
  searchTerm: string | undefined,
  options: SplitOptions = {},
): HighlightSegment[] {
  const { caseInsensitive = false } = options;
  if (!searchTerm) return [{ text, isHighlight: false }];

  const haystack = caseInsensitive ? text.toLocaleLowerCase() : text;
  const needle = caseInsensitive ? searchTerm.toLocaleLowerCase() : searchTerm;
  if (!needle.length) return [{ text, isHighlight: false }];

  const parts: HighlightSegment[] = [];
  let startIndex = 0;
  let matchIndex: number;

  while ((matchIndex = haystack.indexOf(needle, startIndex)) !== -1) {
    if (matchIndex > startIndex) {
      parts.push({ text: text.slice(startIndex, matchIndex), isHighlight: false });
    }

    const matchEnd = matchIndex + needle.length;
    parts.push({ text: text.slice(matchIndex, matchEnd), isHighlight: true });
    startIndex = matchEnd;
  }

  if (startIndex < text.length) {
    parts.push({ text: text.slice(startIndex), isHighlight: false });
  }

  return parts;
}

function applyMiddleEllipsis(parts: HighlightSegment[], maxChars: number): HighlightSegment[] {
  if (maxChars <= 2) {
    return [{ text: '…', isHighlight: false }];
  }

  const totalLength = parts.reduce((sum, part) => sum + part.text.length, 0);
  if (totalLength <= maxChars) {
    return parts;
  }

  const leftChars = Math.floor((maxChars - 1) / 2); // Reserve one slot for the ellipsis glyph.
  const rightChars = maxChars - leftChars - 1;

  // Populate the leading slice (stop once we run out of space).
  const leftParts: HighlightSegment[] = [];
  let leftCount = 0;
  for (const part of parts) {
    const remainingSpace = leftChars - leftCount;
    if (remainingSpace <= 0) break;

    if (part.text.length <= remainingSpace) {
      leftParts.push(part);
      leftCount += part.text.length;
    } else {
      leftParts.push({
        text: part.text.slice(0, remainingSpace),
        isHighlight: part.isHighlight,
      });
      break;
    }
  }

  // Populate the trailing slice (build from the end backwards).
  const rightParts: HighlightSegment[] = [];
  let rightCount = 0;
  for (let i = parts.length - 1; i >= 0; i--) {
    const part = parts[i];
    const remainingSpace = rightChars - rightCount;
    if (remainingSpace <= 0) break;

    if (part.text.length <= remainingSpace) {
      rightParts.unshift(part);
      rightCount += part.text.length;
    } else {
      rightParts.unshift({
        text: part.text.slice(-remainingSpace),
        isHighlight: part.isHighlight,
      });
      break;
    }
  }

  return [...leftParts, { text: '…', isHighlight: false }, ...rightParts];
}

type MiddleEllipsisHighlightProps = {
  text: string;
  className?: string;
  highlightTerm?: string;
  caseInsensitive?: boolean;
};

export function MiddleEllipsisHighlight({
  text,
  className,
  highlightTerm,
  caseInsensitive,
}: MiddleEllipsisHighlightProps): React.JSX.Element {
  const containerRef = useRef<HTMLSpanElement | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  // Break the string into highlight + non-highlight chunks only when inputs change.
  const highlightedParts = useMemo(() => {
    return text ? splitTextWithHighlight(text, highlightTerm, { caseInsensitive }) : [];
  }, [text, highlightTerm, caseInsensitive]);

  // Replace the middle of the string with an ellipsis so we preserve both ends.
  const displayParts = useMemo(() => {
    if (!containerWidth || highlightedParts.length === 0) return highlightedParts;

    const maxChars = Math.floor(containerWidth / CHAR_WIDTH) - 1;
    return applyMiddleEllipsis(highlightedParts, maxChars);
  }, [highlightedParts, containerWidth]);

  // Prefer a ResizeObserver so truncation reacts quickly to layout shifts.
  const updateWidth = useCallback(() => {
    const el = containerRef.current;
    if (el) {
      const newWidth = el.getBoundingClientRect().width;
      setContainerWidth(newWidth);
    }
  }, []);

  useEffect(() => {
    updateWidth();

    const resizeObserver = new ResizeObserver(updateWidth);
    const el = containerRef.current;
    if (el) resizeObserver.observe(el);

    return () => resizeObserver.disconnect();
  }, [updateWidth]);

  return (
    <span
      ref={containerRef}
      className={className}
      title={text}
      style={{ display: 'block', width: '100%' }}
    >
      {displayParts.map((part, index) =>
        part.isHighlight ? (
          <strong key={`${part.text}-${index}`}>{part.text}</strong>
        ) : (
          <span key={`${part.text}-${index}`}>{part.text}</span>
        ),
      )}
    </span>
  );
}
