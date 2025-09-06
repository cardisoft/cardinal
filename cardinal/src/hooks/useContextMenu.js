import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// 简化的上下文菜单 hook，统一处理两种菜单类型
export function useContextMenu(autoFitColumns = null) {
  const [menu, setMenu] = useState({
    visible: false,
    x: 0,
    y: 0,
    type: null,
    data: null
  });

  // 统一的菜单显示函数
  const showMenu = useCallback((e, type, data = null) => {
    e.preventDefault();
    e.stopPropagation();
    setMenu({
      visible: true,
      x: e.clientX,
      y: e.clientY,
      type,
      data
    });
  }, []);

  // 文件菜单
  const showContextMenu = useCallback((e, path) => {
    showMenu(e, 'file', path);
  }, [showMenu]);

  // 头部菜单
  const showHeaderContextMenu = useCallback((e) => {
    showMenu(e, 'header');
  }, [showMenu]);

  const closeMenu = useCallback(() => {
    setMenu(prev => ({ ...prev, visible: false }));
  }, []);

  // 根据类型生成菜单项
  const getMenuItems = () => {
    if (menu.type === 'file') {
      return [
        {
          label: 'Open in Finder',
          action: () => invoke('open_in_finder', { path: menu.data }),
        },
        {
          label: 'Copy Path',
          action: () => navigator.clipboard.writeText(menu.data),
        },
      ];
    }
    if (menu.type === 'header' && autoFitColumns) {
      return [{
        label: 'Reset Column Widths',
        action: autoFitColumns,
      }];
    }
    return [];
  };

  return {
    menu,
    showContextMenu,
    showHeaderContextMenu,
    closeMenu,
    getMenuItems
  };
}
