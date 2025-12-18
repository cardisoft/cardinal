import React, { useEffect, useRef, useState, useMemo } from 'react';

const CHAR_WIDTH = 8; // Approximate monospace character width in pixels – used for quick truncation math.

export type HighlightSegment = {
  text: string;
  isHighlight: boolean;
};

type SplitOptions = {
  caseInsensitive?: boolean;
};

export function splitTextWithHighlights(
  text: string,
  searchTerms: readonly string[] | undefined,
  options: SplitOptions = {},
): HighlightSegment[] {
  const { caseInsensitive = false } = options;
  if (!text) return [];

  const needles = (searchTerms ?? [])
    .map((term) => term?.trim())
    .filter((term): term is string => Boolean(term))
    .map((term) => (caseInsensitive ? term.toLocaleLowerCase() : term));

  if (needles.length === 0) {
    return [{ text, isHighlight: false }];
  }

  const haystack = caseInsensitive ? text.toLocaleLowerCase() : text;
  const highlightMask = Array<boolean>(text.length).fill(false);
  for (const needle of needles) {
    if (!needle.length) continue;
    let searchIndex = 0;
    while (searchIndex <= haystack.length) {
      const matchIndex = haystack.indexOf(needle, searchIndex);
      if (matchIndex === -1) break;
      for (let offset = 0; offset < needle.length; offset++) {
        highlightMask[matchIndex + offset] = true;
      }
      searchIndex = matchIndex + needle.length;
    }
  }

  if (!highlightMask.some(Boolean)) {
    return [{ text, isHighlight: false }];
  }

  const parts: HighlightSegment[] = [];
  let segmentStart = 0;
  let segmentState = Boolean(highlightMask[0]);

  for (let index = 1; index < text.length; index++) {
    const nextState = Boolean(highlightMask[index]);
    if (nextState === segmentState) {
      continue;
    }
    if (index > segmentStart) {
      parts.push({
        text: text.slice(segmentStart, index),
        isHighlight: segmentState,
      });
    }
    segmentStart = index;
    segmentState = nextState;
  }

  if (segmentStart < text.length) {
    parts.push({
      text: text.slice(segmentStart),
      isHighlight: segmentState,
    });
  }

  return parts;
}

export function applyMiddleEllipsis(
  parts: HighlightSegment[],
  maxChars: number,
): HighlightSegment[] {
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

  // Trim the parts adjacent to the ellipsis to avoid awkward spacing
  if (leftParts.length > 0) {
    const lastPart = leftParts[leftParts.length - 1];
    leftParts[leftParts.length - 1] = { ...lastPart, text: lastPart.text.trimEnd() };
  }
  if (rightParts.length > 0) {
    const firstPart = rightParts[0];
    rightParts[0] = { ...firstPart, text: firstPart.text.trimStart() };
  }

  return [...leftParts, { text: '…', isHighlight: false }, ...rightParts];
}

type MiddleEllipsisHighlightProps = {
  text: string;
  className?: string;
  highlightTerms?: readonly string[];
  caseInsensitive?: boolean;
};

export function MiddleEllipsisHighlight({
  text,
  className,
  highlightTerms,
  caseInsensitive,
}: MiddleEllipsisHighlightProps): React.JSX.Element {
  const containerRef = useRef<HTMLSpanElement | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  // Break the string into highlight + non-highlight chunks only when inputs change.
  const highlightedParts = useMemo(() => {
    return text ? splitTextWithHighlights(text, highlightTerms, { caseInsensitive }) : [];
  }, [text, highlightTerms, caseInsensitive]);

  // Replace the middle of the string with an ellipsis so we preserve both ends.
  const displayParts = useMemo(() => {
    if (!containerWidth || highlightedParts.length === 0) return highlightedParts;

    const maxChars = Math.floor(containerWidth / CHAR_WIDTH) - 1;
    return applyMiddleEllipsis(highlightedParts, maxChars);
  }, [highlightedParts, containerWidth]);

  // Prefer a ResizeObserver so truncation reacts quickly to layout shifts.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) {
      return;
    }

    const rect = el.getBoundingClientRect();
    setContainerWidth(rect.width);

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      const newWidth = entry.contentRect.width;
      setContainerWidth((prev) => (prev === newWidth ? prev : newWidth));
    });

    resizeObserver.observe(el);
    return () => resizeObserver.disconnect();
  }, []);

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
