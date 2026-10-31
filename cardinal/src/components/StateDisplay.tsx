import React from 'react';
import { useTranslation } from 'react-i18next';

type StateProps = {
  icon: React.ReactNode;
  title: string;
  message?: React.ReactNode;
};

export type DisplayState =
  | 'loading'
  | 'indexing'
  | 'error'
  | 'empty'
  | 'scope-unavailable'
  | 'results';
type EmptyState = Exclude<DisplayState, 'results'>;

type StateDisplayProps = {
  state: EmptyState;
  message?: string | null;
  /** Overrides the default title (used to name the folder being indexed). */
  title?: string | null;
  query?: string;
  directoryQuery?: string;
  /** Folder scope reported as unavailable by the backend. */
  scope?: string | null;
};

const State = ({ icon, title, message }: StateProps): React.JSX.Element => (
  <div className="state-display">
    <div className="state-content">
      <div className="state-icon">{icon}</div>
      <div className="state-title">{title}</div>
      <div className="state-message">{message}</div>
    </div>
  </div>
);

// Consistent empty/error/loading presentation inside the results pane.
export function StateDisplay({
  state,
  message,
  title,
  query,
  directoryQuery,
  scope,
}: StateDisplayProps): React.JSX.Element {
  const { t } = useTranslation();
  if (state === 'loading') {
    return <State icon={<div className="spinner" />} title={title ?? t('stateDisplay.loading')} />;
  }

  if (state === 'indexing') {
    return (
      <State
        icon={<div className="spinner" />}
        title={title ?? t('stateDisplay.indexingTitle')}
        message={message ?? t('stateDisplay.indexingMessage')}
      />
    );
  }

  if (state === 'scope-unavailable') {
    return (
      <State
        icon={<div className="error-icon">!</div>}
        title={t('stateDisplay.scopeUnavailableTitle', { scope: scope ?? directoryQuery ?? '' })}
        message={t('stateDisplay.scopeUnavailableMessage')}
      />
    );
  }

  if (state === 'error') {
    return (
      <State
        icon={<div className="error-icon">!</div>}
        title={t('stateDisplay.error')}
        message={message}
      />
    );
  }

  const icon = (
    <svg
      width="72"
      height="72"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
      <line x1="13" y1="9" x2="9" y2="13" />
      <line x1="9" y1="9" x2="13" y2="13" />
    </svg>
  );
  const emptyTitle =
    query && directoryQuery
      ? t('stateDisplay.emptyTitleWithDirectory', { query, directoryQuery })
      : t('stateDisplay.emptyTitle', { query: query || directoryQuery || '' });
  return <State icon={icon} title={emptyTitle} message={t('stateDisplay.emptyMessage')} />;
}
