import type { CharlotteChatApi } from '../../preload/chat';

declare global {
  interface Window {
    charlotteChat: CharlotteChatApi;
  }
}

export {};
