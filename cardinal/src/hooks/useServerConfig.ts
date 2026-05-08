import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

export type ServerConfig = Readonly<{
  enabled: boolean;
  endpoint: string;
}>;

type StoredServerConfig = Partial<ServerConfig> & {
  port?: number;
};

const STORAGE_KEY = 'cardinal.serverConfig';
const DEFAULT_SERVER_CONFIG: ServerConfig = {
  enabled: false,
  endpoint: '127.0.0.1:3388',
};

export const isValidEndpoint = (endpoint: string): boolean => {
  const trimmed = endpoint.trim();
  if (!trimmed) {
    return false;
  }

  const portSeparator = trimmed.lastIndexOf(':');
  if (portSeparator <= 0 || portSeparator === trimmed.length - 1) {
    return false;
  }

  const port = Number.parseInt(trimmed.slice(portSeparator + 1), 10);
  return Number.isInteger(port) && port >= 1 && port <= 65535;
};

const normalizeServerConfig = (value: ServerConfig): ServerConfig => ({
  enabled: Boolean(value.enabled),
  endpoint: isValidEndpoint(value.endpoint)
    ? value.endpoint.trim()
    : DEFAULT_SERVER_CONFIG.endpoint,
});

export function useServerConfig(): {
  serverConfig: ServerConfig;
  setServerConfig: (next: ServerConfig) => void;
  defaultServerConfig: ServerConfig;
} {
  const [serverConfig, setServerConfigState] = useStoredState<ServerConfig>({
    key: STORAGE_KEY,
    defaultValue: DEFAULT_SERVER_CONFIG,
    read: (raw) => {
      const parsed = JSON.parse(raw) as StoredServerConfig;
      if (typeof parsed !== 'object' || parsed == null) {
        return null;
      }
      if (typeof parsed.endpoint === 'string') {
        return normalizeServerConfig({
          enabled: Boolean(parsed.enabled),
          endpoint: parsed.endpoint,
        });
      }
      return normalizeServerConfig({
        enabled: Boolean(parsed.enabled),
        endpoint:
          typeof parsed.port === 'number'
            ? `127.0.0.1:${parsed.port}`
            : DEFAULT_SERVER_CONFIG.endpoint,
      });
    },
    write: (value) => JSON.stringify(normalizeServerConfig(value)),
    normalize: normalizeServerConfig,
    readErrorMessage: 'Failed to read stored server config preference',
    writeErrorMessage: 'Failed to persist server config preference',
  });

  const setServerConfig = useCallback(
    (next: ServerConfig) => {
      setServerConfigState(next);
    },
    [setServerConfigState],
  );

  return {
    serverConfig,
    setServerConfig,
    defaultServerConfig: DEFAULT_SERVER_CONFIG,
  };
}
