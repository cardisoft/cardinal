import React from 'react';
import ReactDOM from 'react-dom/client';
import './i18n/config';
import App from './App';
import { initializeAppMenu } from './menu';
import { initializeTauriEventRuntime } from './runtime/tauriEventRuntime';
import { initializeGlobalShortcuts } from './utils/globalShortcuts';
import { initializeThemePreference } from './theme';

initializeThemePreference();
void initializeGlobalShortcuts();
void initializeAppMenu();
void initializeTauriEventRuntime();

const rootElement = document.getElementById('root');

if (!rootElement) {
  throw new Error('Unable to initialize application: #root element is missing.');
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
