import { OPEN_PREFERENCES_EVENT } from '../constants/appEvents';

export const openPreferences = (): void => {
  if (typeof window === 'undefined') {
    return;
  }
  window.dispatchEvent(new Event(OPEN_PREFERENCES_EVENT));
};
