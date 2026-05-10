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

if (typeof window !== 'undefined') {
  let localStorageLike: Storage | undefined;
  try {
    localStorageLike = window.localStorage as Storage | undefined;
  } catch {
    localStorageLike = undefined;
  }

  const hasUsableLocalStorage =
    localStorageLike &&
    typeof localStorageLike.getItem === 'function' &&
    typeof localStorageLike.setItem === 'function' &&
    typeof localStorageLike.removeItem === 'function' &&
    typeof localStorageLike.clear === 'function';

  if (!hasUsableLocalStorage) {
    const store = new Map<string, string>();
    const storagePrototype = typeof Storage === 'undefined' ? Object.prototype : Storage.prototype;
    const fallback = Object.create(storagePrototype) as Storage;

    Object.defineProperty(storagePrototype, 'clear', {
      configurable: true,
      value: () => store.clear(),
    });
    Object.defineProperty(storagePrototype, 'getItem', {
      configurable: true,
      value: (key: string) => store.get(key) ?? null,
    });
    Object.defineProperty(storagePrototype, 'key', {
      configurable: true,
      value: (index: number) => Array.from(store.keys())[index] ?? null,
    });
    Object.defineProperty(storagePrototype, 'removeItem', {
      configurable: true,
      value: (key: string) => {
        store.delete(key);
      },
    });
    Object.defineProperty(storagePrototype, 'setItem', {
      configurable: true,
      value: (key: string, value: string) => {
        store.set(key, String(value));
      },
    });

    Object.defineProperty(fallback, 'length', {
      configurable: true,
      get: () => store.size,
    });

    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: fallback,
    });
  }
}
