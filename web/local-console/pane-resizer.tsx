import { type KeyboardEvent, type PointerEvent, useEffect, useRef, useState } from 'react';

type Props = {
  /** CSS custom property on <body> that holds the pane width, e.g. `--chat-w`. */
  cssVar: string;
  label: string;
  min: number;
  max: number;
  initial: number;
  /** 1 when dragging right widens the pane (left-hand pane), -1 for a right-hand pane. */
  direction: 1 | -1;
  className?: string;
  /** Keep a docked pane within this fraction of its containing work area. */
  maxRatio?: number;
};

const key = (cssVar: string) => `gnomcode.pane${cssVar}`;

function readWidth(cssVar: string, fallback: number): number {
  try {
    const saved = Number(localStorage.getItem(key(cssVar)));
    return Number.isFinite(saved) && saved > 0 ? saved : fallback;
  } catch {
    return fallback;
  }
}

/** Drag, arrow-key (Shift = 32px) and Home/double-click-reset handle for a resizable pane. Width is a per-viewer convenience. */
export function PaneResizer({ cssVar, label, min, max, initial, direction, className = '', maxRatio }: Props) {
  const handleRef = useRef<HTMLDivElement>(null);
  const [containerMax, setContainerMax] = useState(max);
  const limit = Math.min(max, containerMax);
  const clamp = (value: number) => Math.round(Math.min(limit, Math.max(min, value)));
  const [width, setWidth] = useState(() => clamp(readWidth(cssVar, initial)));

  useEffect(() => {
    const container = handleRef.current?.parentElement?.parentElement;
    if (!container || !maxRatio) return;
    const measure = () => {
      const pane = handleRef.current?.parentElement;
      if (!pane || !pane.getClientRects().length || getComputedStyle(pane).position === 'absolute') return;
      setContainerMax(Math.max(min, Math.floor(container.clientWidth * maxRatio)));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(container);
    return () => observer.disconnect();
  }, [maxRatio, min]);

  useEffect(() => {
    setWidth(current => Math.min(limit, Math.max(min, current)));
  }, [limit, min]);

  useEffect(() => {
    document.body.style.setProperty(cssVar, `${width}px`);
    try {
      localStorage.setItem(key(cssVar), String(width));
    } catch {
      // Storage unavailable
    }
  }, [cssVar, width]);

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    const handle = event.currentTarget;
    const startX = event.clientX;
    const startW = width;
    handle.setPointerCapture(event.pointerId);
    document.body.classList.add('sp-resizing');
    const move = (next: globalThis.PointerEvent) => setWidth(clamp(startW + (next.clientX - startX) * direction));
    const up = () => {
      document.body.classList.remove('sp-resizing');
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
      handle.removeEventListener('pointercancel', up);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
    handle.addEventListener('pointercancel', up);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.shiftKey ? 32 : 8;
    if (event.key === 'ArrowLeft') setWidth(clamp(width - step * direction));
    else if (event.key === 'ArrowRight') setWidth(clamp(width + step * direction));
    else if (event.key === 'Home') setWidth(clamp(initial));
    else return;
    event.preventDefault();
  };

  return (
    <div
      ref={handleRef}
      className={`sp-pane-resizer ${className}`}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={limit}
      aria-valuenow={width}
      tabIndex={0}
      title={`${label}. Drag, or use the arrow keys. Double-click to reset.`}
      onPointerDown={onPointerDown}
      onKeyDown={onKeyDown}
      onDoubleClick={() => setWidth(clamp(initial))}
    />
  );
}
