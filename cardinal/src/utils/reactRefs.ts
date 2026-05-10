import type { MutableRefObject, Ref } from 'react';

export const assignRef = <T>(ref: Ref<T> | undefined, value: T | null): void => {
  if (!ref) {
    return;
  }

  if (typeof ref === 'function') {
    ref(value);
    return;
  }

  (ref as MutableRefObject<T | null>).current = value;
};
