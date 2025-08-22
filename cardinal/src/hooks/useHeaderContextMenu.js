import { useState, useCallback } from 'react';

export function useHeaderContextMenu(autoFitColumns) {
  const [headerContextMenu, setHeaderContextMenu] = useState({ 
    visible: false, 
    x: 0, 
    y: 0 
  });

  const showHeaderContextMenu = useCallback((e) => {
    e.preventDefault();
    e.stopPropagation();
    setHeaderContextMenu({ 
      visible: true, 
      x: e.clientX, 
      y: e.clientY 
    });
  }, []);

  const closeHeaderContextMenu = useCallback(() => {
    setHeaderContextMenu(prev => ({ ...prev, visible: false }));
  }, []);

  const headerMenuItems = [
    {
      label: 'Reset',
      action: () => {
        autoFitColumns();
      },
    },
  ];

  return {
    headerContextMenu,
    showHeaderContextMenu,
    closeHeaderContextMenu,
    headerMenuItems
  };
}
