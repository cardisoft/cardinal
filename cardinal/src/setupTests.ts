import '@testing-library/jest-dom';

class ResizeObserverStub implements ResizeObserver {
  callback: ResizeObserverCallback;

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
  }

  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

if (typeof window !== 'undefined' && !window.ResizeObserver) {
  window.ResizeObserver = ResizeObserverStub;
}

if (typeof globalThis.ResizeObserver === 'undefined') {
  globalThis.ResizeObserver = ResizeObserverStub;
}

// Node 26 ships a built-in `localStorage` getter that resolves to `undefined` unless the
// process was started with `--localstorage-file`. That getter shadows jsdom's implementation,
// so every hook that persists state (`useStoredState`, `useIgnorePaths`, ...) throws on
// `window.localStorage.getItem`. Install an in-memory implementation on jsdom's prototype
// only when the real storage is missing; keeping it on `Storage.prototype` means tests can
// still spy on `Storage.prototype.setItem`.
if (typeof window !== 'undefined' && !window.localStorage) {
  const backing = new WeakMap<object, Map<string, string>>();
  const dataOf = (self: object): Map<string, string> => {
    let data = backing.get(self);
    if (!data) {
      data = new Map();
      backing.set(self, data);
    }
    return data;
  };

  Storage.prototype.getItem = function getItem(this: Storage, key: string): string | null {
    const data = dataOf(this);
    const value = data.get(String(key));
    return value === undefined ? null : value;
  };
  Storage.prototype.setItem = function setItem(this: Storage, key: string, value: string): void {
    dataOf(this).set(String(key), String(value));
  };
  Storage.prototype.removeItem = function removeItem(this: Storage, key: string): void {
    dataOf(this).delete(String(key));
  };
  Storage.prototype.clear = function clear(this: Storage): void {
    dataOf(this).clear();
  };
  Storage.prototype.key = function key(this: Storage, index: number): string | null {
    const keys = Array.from(dataOf(this).keys());
    return keys[index] ?? null;
  };
  Object.defineProperty(Storage.prototype, 'length', {
    configurable: true,
    get(this: Storage): number {
      return dataOf(this).size;
    },
  });

  const storage = Object.create(Storage.prototype) as Storage;
  Object.defineProperty(globalThis, 'localStorage', {
    value: storage,
    configurable: true,
    writable: true,
  });
  Object.defineProperty(window, 'localStorage', {
    value: storage,
    configurable: true,
    writable: true,
  });
}
