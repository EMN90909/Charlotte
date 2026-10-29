import type { CharlotteChromeApi } from '../../preload/chrome';

declare global {
  interface Window {
    charlotte: CharlotteChromeApi;
  }
}

export {};
