import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';

interface TourStop {
  /** Matches a `data-tour` attribute in Toolbar.tsx. */
  target: string;
  title: string;
  body: string;
}

// Right to left across the toolbar, so the spotlight only ever travels one
// way - starting with page chat, the thing people most often missed.
const STOPS: TourStop[] = [
  {
    target: 'chat',
    title: 'Chat with people on this page',
    body: 'Every web page has its own chat room. Click here to talk with whoever else is reading it. Switch to Agents inside the panel for private AI chats.',
  },
  {
    target: 'settings',
    title: 'Settings',
    body: 'Your username, AI agents, MCP servers, theme, privacy and everything else you set up just now lives here.',
  },
  {
    target: 'downloads',
    title: 'Downloads',
    body: "Files you download show up here. A dot appears while something's still downloading.",
  },
  {
    target: 'translate',
    title: 'Translate this page',
    body: 'Rewrites the whole page in your language using one of your AI agents, and keeps up as the page changes.',
  },
  {
    target: 'subtitles',
    title: 'Live subtitles',
    body: 'Translated captions for any video or audio playing in this tab, as it plays.',
  },
  {
    target: 'bookmark',
    title: 'Bookmark this page',
    body: 'Save the page you’re on. Click again to rename it or move it into a folder.',
  },
  {
    target: 'address',
    title: 'Go anywhere',
    body: 'Type a web address or search. Your history and bookmarks show up as suggestions while you type.',
  },
];

const CARD_WIDTH = 290;
const EDGE = 12;
const SPOT_PAD = 6;

interface Rect {
  top: number;
  left: number;
  width: number;
  height: number;
}

function measure(target: string): Rect | null {
  const el = document.querySelector<HTMLElement>(`[data-tour="${target}"]`);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  if (r.width === 0 && r.height === 0) return null;
  return { top: r.top, left: r.left, width: r.width, height: r.height };
}

/**
 * Coach marks shown once, right after onboarding: the app is dimmed and
 * click-blocked while a spotlight walks across the toolbar, each stop with
 * a short callout pointing at the button. Purely visual - it only reads
 * toolbar positions, never touches toolbar state, and any stop whose button
 * isn't on screen is skipped rather than pointing at nothing.
 */
export function ToolbarTour({ onDone }: { onDone: () => void }) {
  const [stops] = useState(() => STOPS.filter((s) => measure(s.target)));
  const [index, setIndex] = useState(0);
  const [rect, setRect] = useState<Rect | null>(null);
  const nextRef = useRef<HTMLButtonElement>(null);

  const stop = stops[index];
  const isLast = index === stops.length - 1;

  const next = useCallback(() => (isLast ? onDone() : setIndex((i) => i + 1)), [isLast, onDone]);
  const back = useCallback(() => setIndex((i) => Math.max(0, i - 1)), []);

  useLayoutEffect(() => {
    if (!stop) return;
    const update = () => setRect(measure(stop.target));
    update();
    window.addEventListener('resize', update);
    return () => window.removeEventListener('resize', update);
  }, [stop]);

  useEffect(() => {
    if (stops.length === 0) onDone();
  }, [stops.length, onDone]);

  useEffect(() => {
    nextRef.current?.focus();
  }, [index]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onDone();
      else if (e.key === 'ArrowRight') next();
      else if (e.key === 'ArrowLeft') back();
      else return;
      e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [next, back, onDone]);

  if (!stop || !rect) return null;

  const centerX = rect.left + rect.width / 2;
  const cardLeft = Math.min(Math.max(centerX - CARD_WIDTH / 2, EDGE), window.innerWidth - CARD_WIDTH - EDGE);
  const arrowLeft = Math.min(Math.max(centerX - cardLeft, 22), CARD_WIDTH - 22);
  const cardTop = rect.top + rect.height + SPOT_PAD + 16;

  return (
    <div className="tour" role="dialog" aria-modal="true" aria-labelledby="tour-title" aria-describedby="tour-body">
      <div
        className="tour__spot"
        style={{
          top: rect.top - SPOT_PAD,
          left: rect.left - SPOT_PAD,
          width: rect.width + SPOT_PAD * 2,
          height: rect.height + SPOT_PAD * 2,
        }}
      />
      <div className="tour__card" style={{ top: cardTop, left: cardLeft, width: CARD_WIDTH }} key={stop.target}>
        <span className="tour__arrow" style={{ left: arrowLeft }} aria-hidden />
        <span className="tour__count">
          {index + 1} / {stops.length}
        </span>
        <h2 id="tour-title">{stop.title}</h2>
        <p id="tour-body">{stop.body}</p>
        <div className="tour__actions">
          <button type="button" className="tour__skip" onClick={onDone}>
            {isLast ? 'Close' : 'Skip tour'}
          </button>
          <div className="tour__nav">
            {index > 0 && (
              <button type="button" className="tour__back" onClick={back}>
                Back
              </button>
            )}
            <button type="button" ref={nextRef} className="tour__next" onClick={next}>
              {isLast ? 'Got it' : 'Next'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
