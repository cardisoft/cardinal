import { OPEN_BOOKMARKS_EVENT } from '../constants/appEvents';

export const openBookmarks = (): void => {
  if (typeof window === 'undefined') {
    return;
  }
  window.dispatchEvent(new Event(OPEN_BOOKMARKS_EVENT));
};
