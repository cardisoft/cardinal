import React from 'react';
import ReactDOM from 'react-dom/client';
// Side-effect import first: i18n must be initialised before any component renders.
import i18n, { LANGUAGE_STORAGE_KEY } from './i18n/config';
import App from './App';
import BookmarksWindow from './components/BookmarksWindow';
import { initializeAppMenu } from './menu';
import { initializeTauriEventRuntime } from './runtime/tauriEventRuntime';
import { initializeGlobalShortcuts } from './utils/globalShortcuts';
import { applyStoredUiPreferences, subscribeUiPreferencesChanged } from './runtime/uiPreferences';

// Both windows must apply the same theme/font/language, and both must react when the other
// window changes them (each webview paints only its own document).
const applyPersistedLanguage = () => {
  try {
    const stored = window.localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (stored && stored !== i18n.language) {
      void i18n.changeLanguage(stored);
    }
  } catch {
    // Storage is best-effort; a failure only means the language stays as it is.
  }
};

applyStoredUiPreferences({ onAfterApply: applyPersistedLanguage });
subscribeUiPreferencesChanged({ onAfterApply: applyPersistedLanguage });

// One bundle, two windows: `index.html?window=bookmarks` boots the bookmarks manager alone.
// The main window keeps the menu, global hotkey and event runtime to itself.
const windowKind = new URLSearchParams(window.location.search).get('window');
const isBookmarksWindow = windowKind === 'bookmarks';

if (!isBookmarksWindow) {
  void initializeGlobalShortcuts();
  void initializeAppMenu();
  void initializeTauriEventRuntime();
}

const rootElement = document.getElementById('root');

if (!rootElement) {
  throw new Error('Unable to initialize application: #root element is missing.');
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>{isBookmarksWindow ? <BookmarksWindow /> : <App />}</React.StrictMode>,
);
