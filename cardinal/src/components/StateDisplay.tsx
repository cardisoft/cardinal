import React from 'react';

type StateProps = {
  icon: React.ReactNode;
  title: string;
  message?: React.ReactNode;
};

type StateDisplayProps = {
  state: 'loading' | 'error' | 'empty' | 'results';
  message?: string | null;
  query?: string;
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
export function StateDisplay({ state, message, query }: StateDisplayProps): React.JSX.Element | null {
  if (state === 'loading') {
    return <State icon={<div className="spinner" />} title="Searching..." />;
  }

  if (state === 'error') {
    return (
      <State icon={<div className="error-icon">!</div>} title="Search Error" message={message} />
    );
  }

  if (state === 'empty') {
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
    return (
      <State
        icon={icon}
        title={`No results for "${query}"`}
        message="Try adjusting your keywords or filters."
      />
    );
  }

  return null;
}
