import React from 'react';

type PermissionOverlayProps = {
  title: string;
  description: string;
  steps: string[];
  statusMessage: string | null;
  onRequestPermission: () => void;
  /** Re-probes the permission right now; the hook also polls automatically. */
  onRetry: () => void;
  retryLabel: string;
  /** Hint explaining that access is detected automatically once granted. */
  autoDetectHint: string;
  disabled: boolean;
  actionLabel: string;
};

export function PermissionOverlay({
  title,
  description,
  steps,
  statusMessage,
  onRequestPermission,
  onRetry,
  retryLabel,
  autoDetectHint,
  disabled,
  actionLabel,
}: PermissionOverlayProps): React.JSX.Element {
  return (
    <div className="permission-overlay">
      <div className="permission-card" role="dialog" aria-modal="true">
        <h1>{title}</h1>
        <p>{description}</p>
        <ol>
          {steps.map((step, index) => (
            <li key={index}>{step}</li>
          ))}
        </ol>
        <p className="permission-status" role="status" aria-live="polite">
          {statusMessage}
        </p>
        <p className="permission-hint">{autoDetectHint}</p>
        <div className="permission-actions">
          <button type="button" onClick={onRequestPermission} disabled={disabled}>
            {actionLabel}
          </button>
          <button type="button" className="permission-retry" onClick={onRetry} disabled={disabled}>
            {retryLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
