import { useCallback, useEffect, useRef, useState } from 'react';
import {
  checkFullDiskAccessPermission,
  requestFullDiskAccessPermission as requestNativeFullDiskAccessPermission,
} from 'tauri-plugin-macos-permissions-api';

export type FullDiskAccessStatus = 'granted' | 'denied';

/** Upper bound for a single macOS permission probe (see `refreshStatus`). */
export const FDA_CHECK_TIMEOUT_MS = 4000;

/**
 * How often to re-probe while access is denied. The probe is a real directory read, so macOS
 * re-evaluates the TCC decision each time: granting Full Disk Access in System Settings is
 * picked up within one interval, without restarting the app.
 */
export const FDA_POLL_INTERVAL_MS = 1200;

type UseFullDiskAccessPermissionResult = {
  status: FullDiskAccessStatus;
  isChecking: boolean;
  requestPermission: () => Promise<void>;
  /** Re-probes immediately (used by the overlay's "check again" action). */
  recheck: () => Promise<void>;
};

type ProbeResult = { timedOut: boolean; authorized: boolean };

// Centralise macOS Full Disk Access state so App.tsx stays focused on UI concerns.
export function useFullDiskAccessPermission(): UseFullDiskAccessPermissionResult {
  const [status, setStatus] = useState<FullDiskAccessStatus>('granted');
  const [isChecking, setIsChecking] = useState(true);
  const hasLoggedPermissionStatusRef = useRef(false);
  const inFlightRef = useRef(false);

  const probe = useCallback(async (): Promise<ProbeResult> => {
    try {
      return await Promise.race([
        checkFullDiskAccessPermission().then((authorized) => ({ timedOut: false, authorized })),
        // A hung probe would leave the app on the loading screen forever; bail out and keep
        // whatever status we already have instead of reporting a false negative.
        new Promise<ProbeResult>((resolve) =>
          setTimeout(() => resolve({ timedOut: true, authorized: false }), FDA_CHECK_TIMEOUT_MS),
        ),
      ]);
    } catch (error) {
      console.error('Failed to check full disk access permission', error);
      return { timedOut: false, authorized: false };
    }
  }, []);

  const refreshStatus = useCallback(
    async (options?: { silent?: boolean }) => {
      // Overlapping probes would let an older result overwrite a newer one.
      if (inFlightRef.current) {
        return;
      }
      inFlightRef.current = true;
      if (!options?.silent) {
        setIsChecking(true);
      }

      try {
        const result = await probe();
        if (result.timedOut) {
          console.warn('Full Disk Access check timed out; keeping the current status');
          return;
        }

        if (!hasLoggedPermissionStatusRef.current) {
          console.log('Full Disk Access granted:', result.authorized);
          hasLoggedPermissionStatusRef.current = true;
        }
        setStatus(result.authorized ? 'granted' : 'denied');
      } finally {
        inFlightRef.current = false;
        if (!options?.silent) {
          setIsChecking(false);
        }
      }
    },
    [probe],
  );

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  // While denied, keep watching for the user granting access in System Settings. Polling is
  // cheap (two directory reads) and focus/visibility changes give an instant response when
  // the user switches back to the app.
  useEffect(() => {
    if (status !== 'denied') {
      return;
    }

    const poll = () => void refreshStatus({ silent: true });
    const interval = setInterval(poll, FDA_POLL_INTERVAL_MS);

    const handleFocus = () => {
      if (document.visibilityState === 'visible') {
        poll();
      }
    };
    window.addEventListener('focus', handleFocus);
    document.addEventListener('visibilitychange', handleFocus);

    return () => {
      clearInterval(interval);
      window.removeEventListener('focus', handleFocus);
      document.removeEventListener('visibilitychange', handleFocus);
    };
  }, [refreshStatus, status]);

  const requestPermission = useCallback(async () => {
    try {
      await requestNativeFullDiskAccessPermission();
    } finally {
      // The check right after opening Settings still reports "denied"; the poll above takes
      // over from there.
      await refreshStatus({ silent: true });
    }
  }, [refreshStatus]);

  const recheck = useCallback(async () => {
    await refreshStatus();
  }, [refreshStatus]);

  return {
    status,
    isChecking,
    requestPermission,
    recheck,
  };
}
