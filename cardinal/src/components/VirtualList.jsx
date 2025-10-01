// React & libs
import React, {
	useRef,
	useState,
	useCallback,
	useLayoutEffect,
	useEffect,
	forwardRef,
	useImperativeHandle
} from 'react';
import { invoke } from '@tauri-apps/api/core';
import Scrollbar from './Scrollbar';
import { useDataLoader } from '../hooks/useDataLoader';

/**
 * 虚拟滚动列表组件（含行数据按需加载缓存）
 */
export const VirtualList = forwardRef(function VirtualList({
	results = null,
	rowHeight = 24,
	overscan = 5,
	renderRow,
	onScrollSync,
	className = ''
}, ref) {
	// ----- refs -----
	const containerRef = useRef(null);
	const iconRequestIdRef = useRef(0);

	// ----- state -----
	const [scrollTop, setScrollTop] = useState(0);
	const [viewportHeight, setViewportHeight] = useState(0);

	// ----- derived -----
	// 行数直接来自 results（不再支持显式 rowCount）
	const rowCount = results?.length ?? 0;

	// ----- data loader -----
	const { cache, ensureRangeLoaded } = useDataLoader(results);

	// 计算总虚拟高度和滚动范围
	const totalHeight = rowCount * rowHeight;
	const maxScrollTop = Math.max(0, totalHeight - viewportHeight);

	// ----- callbacks: pure calculations first -----
	// 计算可见范围
	const start = rowCount && viewportHeight
		? Math.max(0, Math.floor(scrollTop / rowHeight) - overscan)
		: 0;
	const end = rowCount && viewportHeight
		? Math.min(rowCount - 1, Math.ceil((scrollTop + viewportHeight) / rowHeight) + overscan - 1)
		: -1;

	// 更新滚动位置
	const updateScrollAndRange = useCallback((nextScrollTop) => {
		const clamped = Math.max(0, Math.min(nextScrollTop, maxScrollTop));
		setScrollTop(prev => (prev === clamped ? prev : clamped));
	}, [maxScrollTop]);

	// ----- event handlers -----
	// 垂直滚动（阻止默认以获得一致行为）
	const handleWheel = useCallback((e) => {
		e.preventDefault();
		updateScrollAndRange(scrollTop + e.deltaY);
	}, [scrollTop, updateScrollAndRange]);

	// 水平滚动同步
	const handleHorizontalScroll = useCallback((e) => {
		if (onScrollSync) onScrollSync(e.target.scrollLeft);
	}, [onScrollSync]);

	// ----- effects -----
	const updateIconViewport = useCallback((viewport) => {
		const requestId = iconRequestIdRef.current + 1;
		iconRequestIdRef.current = requestId;
		// console.log('Updating icon viewport', requestId, viewport );
		invoke('update_icon_viewport', { id: requestId, viewport }).catch(error => {
			console.error('Failed to update icon viewport', error);
		});
	}, []);

	// 可见窗口变化时自动加载
	useEffect(() => { // auto load
		if (end >= start) ensureRangeLoaded(start, end);
	}, [start, end, ensureRangeLoaded]);

	useEffect(() => {
		if (!Array.isArray(results) || results.length === 0 || end < start) {
			updateIconViewport([]);
			return;
		}

		const clampedStart = Math.max(0, start);
		const clampedEnd = Math.min(end, results.length - 1);
		if (clampedEnd < clampedStart) {
			updateIconViewport([]);
			return;
		}

		const viewport = results
			.slice(clampedStart, clampedEnd + 1);

		updateIconViewport(viewport);
	}, [results, start, end, updateIconViewport]);

	useEffect(() => () => {
		updateIconViewport([]);
	}, [updateIconViewport]);

	// 监听容器尺寸变化
	useLayoutEffect(() => { // observe container height
		const container = containerRef.current;
		if (!container) return;
		const updateViewport = () => setViewportHeight(container.clientHeight);
		const resizeObserver = new ResizeObserver(updateViewport);
		resizeObserver.observe(container);
		updateViewport();
		return () => resizeObserver.disconnect();
	}, []);

	// 当滚动范围变化时校正位置
	useEffect(() => {
		setScrollTop(prev => {
			const clamped = Math.max(0, Math.min(prev, maxScrollTop));
			return clamped === prev ? prev : clamped;
		});
	}, [maxScrollTop]);

	// ----- imperative API -----
	// 暴露的API
	useImperativeHandle(ref, () => ({
		scrollToTop: () => updateScrollAndRange(0),
		ensureRangeLoaded,
	}), [updateScrollAndRange, ensureRangeLoaded]);

	// ----- rendered items memo -----
	// 渲染的项目
	const baseTop = start * rowHeight - scrollTop;
	const renderedItems = end >= start
		? Array.from({ length: end - start + 1 }, (_, i) => {
			const rowIndex = start + i;
			const item = cache.get(rowIndex);
			return renderRow(rowIndex, item, {
				position: 'absolute',
				top: baseTop + i * rowHeight,
				height: rowHeight,
				left: 0,
				right: 0
			});
		})
		: null;

	// ----- render -----
	return (
		<div
			ref={containerRef}
			className={`virtual-list ${className}`}
			onWheel={handleWheel}
			role="list"
			aria-rowcount={rowCount}
		>
			<div
				className="virtual-list-viewport"
				onScroll={handleHorizontalScroll}
			>
				<div className="virtual-list-items">
					{renderedItems}
				</div>
			</div>
			<Scrollbar
				totalHeight={totalHeight}
				viewportHeight={viewportHeight}
				maxScrollTop={maxScrollTop}
				scrollTop={scrollTop}
				onScrollUpdate={updateScrollAndRange}
			/>
		</div>
	);
});

VirtualList.displayName = 'VirtualList';

export default VirtualList;