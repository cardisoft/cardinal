# Quick Look 优化规划

## 1. 现状总结

- **功能层面**: Cardinal 当前通过 Tauri 指令将文件路径推送给 `QLPreviewPanel`，实现了基础的预览功能。
- **控制层面**: `PreviewController` 作为数据源和代理，通过主线程的 `thread_local!` 管理，但缺少丰富的生命周期和事件管理。
- **体验层面**:
  - 前端仅在按下空格时**打开**面板，缺少“切换”语义，也无法感知面板的关闭状态。
  - 面板动画、键盘控制、当前预览项与UI列表的同步缺失，与系统 Finder 的原生体验差距明显。

## 2. 核心目标

1.  **原生体验对齐**: 实现平滑的缩放动画、键盘导航等，让 Quick Look 的交互体感与系统原生行为一致。
2.  **双向状态同步**: 建立前端 UI 与原生 Quick Look 面板之间的状态同步机制，确保两者状态始终一致。
3.  **提升交互健壮性**: 优化指令设计，处理各种边界情况，提供更可靠、更符合直觉的用户体验。

## 3. 优化实施计划

### Phase 2: 原生视觉与动画打磨

此阶段专注于提升视觉效果，达到“原生感”。

**2.1. 实现平滑的缩放动画**
- **问题**: 面板生硬地出现，没有从列表项“放大”的动画效果。
- **方案**:
  - **前端**: 触发 `toggle_quicklook` 时，使用 `element.getBoundingClientRect()` 获取选中项的 DOM 坐标，并将其作为参数传递给 Rust。
  - **Rust**:
    - 在 `PreviewController` 中存储每个文件路径及其对应的屏幕坐标 (`tauri::Rect`)。
    - 实现 `QLPreviewPanelDelegate` 的 `previewPanel:sourceFrameOnScreenForPreviewItem:` 方法。
    - 在该方法中，根据 `item` 的 `previewItemURL` 查找到其坐标，结合窗口位置计算出**屏幕绝对坐标**并返回。

**2.2. (可选) 提供过渡图像**
- **问题**: 动画过程中可能出现短暂的白色闪烁。
- **方案**:
  - 如果前端有文件图标或缩略图的缓存（例如 Base64 或 URL），可以将其一并传递给 Rust。
  - 在 Rust 中实现 `previewPanel:transitionImageForPreviewItem:contentRect:` 方法，返回对应的 `NSImage`，以提供更平滑的过渡效果。

### Phase 3: 高级交互

此阶段实现更精细的交互控制。

**3.1. 实现键盘事件透传**
- **问题**: Quick Look 激活时，它会成为 Key Window，导致文件列表无法响应方向键等导航快捷键。
- **方案**:
  - 在 `PreviewController` 中实现 `QLPreviewPanelDelegate` 的 `handleEvent:` 方法。
  - 捕获关键的键盘事件（如上/下箭头、删除键等）。
  - 判断事件类型后，通过 `app_handle.emit_all("quicklook-keydown", ...)` 将事件信息转发给前端。
  - 前端监听此事件，并执行对应的列表导航或操作逻辑，从而实现 Quick Look 面板对文件列表的“遥控”。

## 4. 预期收益

- **交互一致性**: 提供与系统 Finder 高度一致的交互与动画体验。
- **状态可靠性**: 通过双向事件同步，确保面板与前端状态一致，减少用户困惑和无效的后台调用。
- **架构融合度**: 通过键盘事件透传和几何信息传递，将原生 Quick Look 面板更无缝地融入到现有的虚拟列表架构中。
