import { useCallback, useState } from 'react';
import type { FileOperationUpdate } from './useContextMenu';

export const mergeFileOperation = (
  current: FileOperationUpdate[],
  operation: FileOperationUpdate,
): FileOperationUpdate[] =>
  [operation, ...current.filter((item) => item.id !== operation.id)].slice(0, 10);

export function useFileOperationLog() {
  const [fileOperations, setFileOperations] = useState<FileOperationUpdate[]>([]);
  const updateFileOperation = useCallback((operation: FileOperationUpdate) => {
    setFileOperations((current) => mergeFileOperation(current, operation));
  }, []);
  return { fileOperations, updateFileOperation };
}
