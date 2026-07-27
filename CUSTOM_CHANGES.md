# 自定义分支改动说明

本文档记录 `my-custom` 分支相对于上游 Cardinal 的主要定制内容。

## 多搜索标签页

- 搜索界面支持同时打开多个标签页，每个标签页独立维护查询、结果和交互状态。
- 点击标签栏中的 `+` 或按 `Command + T` 可以新建搜索标签页。
- 点击标签页的关闭按钮、按 `Command + W`，或使用“窗口 > 关闭标签页”菜单可以关闭当前标签页。
- 关闭最后一个标签页时会自动创建一个新的空白搜索标签页。
- 标签页标题会随当前搜索内容更新。
- 偏好设置和全磁盘访问权限提示仍由所有标签页共享。

## 层级搜索结果

- 当搜索结果中的目录同时也是其他匹配项的祖先目录时，结果会按 Finder 风格显示为可展开的层级结构。
- 目录行提供展开和折叠控件，并通过缩进表示文件层级。
- 为避免大型结果集产生额外的元数据读取和树构建开销，结果超过 20,000 条时继续使用原有的扁平列表。
- 无法读取节点信息时会自动回退到扁平列表。

## Finder 右键搜索

macOS Finder Sync 扩展为 Finder 选中项增加“在 Cardinal 中搜索”右键菜单。

操作流程：

1. 在 Finder 中选中文件或目录。
2. 从右键菜单选择“在 Cardinal 中搜索”。
3. 扩展移除文件扩展名，并通过 `cardinal://finder-search?query=...` 深层链接唤起 Cardinal。
4. Cardinal 为该查询创建并激活一个新的搜索标签页。

相关实现包括：

- `cardinal/src-tauri/finder-sync/`：Finder Sync 扩展源码、权限配置和构建脚本。
- `cardinal/src/runtime/tauriEventRuntime.ts`：深层链接接收与分发。
- `cardinal/src/components/TabManager.tsx`：根据 Finder 请求创建标签页。

## 搜索栏调整

- 搜索文本框和目录范围输入框使用标签页独立的元素 ID，避免多标签页下的 ID 冲突。
- 调整目录范围控件的位置、方向键切换焦点的行为和视觉样式。
- 更新相关测试与所有现有语言资源中的标签页文案。

## macOS 构建说明

- 最低支持版本为 macOS 12。
- `npm run tauri ...` 会先运行 `npm run build:finder-sync` 构建 Finder Sync 扩展。
- Finder Sync 扩展会被打包到应用的 `PlugIns/CardinalFinderSync.appex`。
- 当前扩展构建脚本和 Cargo linker 配置面向 `aarch64-apple-darwin`。
- 扩展使用临时签名；发布构建需要根据实际发行证书和签名流程调整。
- Finder Sync 构建输出位于 `cardinal/src-tauri/finder-sync/build/`，该目录不会提交到 Git。

## 主要依赖与配置

- 增加 `@tauri-apps/plugin-deep-link` 和对应的 Rust/Tauri 配置。
- 注册 `cardinal` URL scheme，并授予深层链接 capability。
- 增加 `tools/clang-rust-lld.sh`，供 Apple Silicon Rust 构建选择对应的 linker。

