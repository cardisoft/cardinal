# 优化待办

## 高优先级
- **为 QuickLook 图标复用现有节点缓存**  
  `cardinal/src-tauri/src/lib.rs` 中的 `cache.expand_file_nodes(&viewport)` 会重新抓取元数据。可以新增轻量接口仅返回路径，复用 `get_nodes_info` 的结果，减少磁盘 IO。
- **节流 icon viewport IPC**  
  `cardinal/src/components/VirtualList.jsx` 在每次滚动时都会 `invoke('update_icon_viewport')`，频繁跨进程。可用 `requestAnimationFrame` 或最小时间窗口批量发送，减少 Tauri IPC 开销并成批触发 QuickLook 生成。
- **缓存 slab 绝对路径**  
  `search-cache/src/cache.rs` 目前在 `expand_file_nodes_inner` 中逐层拼接 `PathBuf`，搜索结果越多越耗时。为 `SlabNode` 缓存绝对路径或父链片段，并在 FSEvent 更新时增量维护，可显著降低分配与遍历成本。

## 中优先级
- **批量调度 icon 生成**  
  当前对每个视窗节点单独 `rayon::spawn`，Viewport 大时会产生大量任务，可换成 `icon_jobs.into_par_iter()` 或固定线程池批量处理，降低调度开销。
- **NamePool 检索结构升级**  
  `namepool/src/lib.rs` 里的 `BTreeSet` + `Mutex` 每次查询都会全表扫描并复制集合。考虑换成 `RwLock` + `fst`/前缀树，或为常见前缀/后缀维护辅助索引，可显著加速模糊搜索并减少锁竞争。
- **优化 icon 更新批处理延迟**  
  `cardinal/src-tauri/src/lib.rs` 的图标线程固定睡眠 100 ms 再批量 drain 队列，造成额外延迟。可改用 `recv_timeout`/`try_recv` 在队列空时即刻 flush，同时在高峰期合并多条消息。
- **减少 QuickLook icon 元数据重复读取**  
  `cardinal/src-tauri/src/lib.rs` 下载 viewport 图标时调用 `cache.expand_file_nodes`，即使不需要 metadata 也会带回整个结构。设计仅返回路径的接口可降低 CPU 与 IPC 开销。
- **避免重复 search 调用**  
  `cardinal/src/App.jsx` 切换搜索选项时会重复触发相同查询。缓存最近一次 `[query, options]`，在完全相同的请求下 short-circuit，可以减少 Tauri 调用与 UI 抖动。
- **缓存 QuickLook 图标编码结果**  
  `cardinal/src-tauri/src/lib.rs` 对同一路径重复执行 QuickLook/NSWorkspace 编码。引入基于 `(path, mtime)` 的 LRU 缓存，可复用 base64 图标数据，降低 CPU 与磁盘负载。
- **后台补全文件元数据**  
  README 中的“metadata 启动后自动 fetching”尚未实现。可在 `SearchCache` 内新增低优先级任务队列，利用 Rayon 后台填充缺失的 ctime/mtime/size，同时保持前端首次渲染快速。



# 备忘录
- 文件空格预览支持
- 支持普通搜索，正则搜索, glob search
    - 不同的格式有开关按钮，类似于 vscode 搜索
    - 在输入不同的内容的时候自动推断是 glob 还是正则还是普通 substr 搜索，然后对应的按钮变成浅色
        - 用户可以手动勾选对应的模式按钮，变成深色
- 搜索结果自动更新
    - FSEvent 更新之后要重新拉取搜索结果
- 搜索结果排序
+ shareded-slab + parking_lot::Mutex(+1 byte，内存体积友好)
    + 问题在于并行读写的正确处理，如 parent 消失场景
        + fsevent 改 slab 结构， metadata fetching 只增添 metadata 不改 slab 结构
+ 加一个页面放 fsevent 列表
    + 知道文件系统有哪些更新的需求
+ 加一个页面搜索一个文件夹曾经有过哪些文件
    + 误删除确认需求
        - 目前 fsevent 处理慢:
            - 并行化 fsevent 的fs遍历部分，写数据部分抢锁
            - 只要有足够快速的 fsevent 处理，我们就可以做到搜索曾经曾经存在过的文件
            - 好像速度也能接受
+ 加一个页面渲染文件体积 treemap, 支持钻取（点击文件夹展开子项）。
    + 磁盘整理需求
+ 申请 macos 全盘访问权限: https://github.com/ayangweb/tauri-plugin-macos-permissions
+ 考虑类LSM/WAL设计?
+ 重启+reopen window之后会很慢
+ viewport range + icon generation pool
+ UI 搜索特定文件夹下的文件
+ 拖动进UI后添加文件路径
## Pending
- fs-icon 仍差强人意，目前显示的和 finder 仍然不一致，怀疑Finder用的别的预览 API(QLThumbnailGenerator?)
    + 改成 QLThumbnailGenerator + NSWorkspace 之后仍然和Finder实现不一样，尽力了尽力了
- 需要在内容没有返回时阻塞滚动条
    - 标准的 scroll bar 没有实现这个，只能用假 scroll bar 了 https://github.com/yairEO/fakescroll/blob/master/react.fakescroll.js
    - AI 写半天写不好，先缓缓
- 搜索结果path中query部分高亮(优化成Rust实现返回matched part)
    - 不高优
- metadata 启动后自动 fetching
    - 目前 metadata replenish 流程不太明朗，先搁置
- metadata 索引
    - ctime, mtime, size
    - 目前 metadata 启动后自动 fetching 流程不太明朗(需要更细粒度的锁)，先搁置
- Windows/NTFS 支持
- /Users/0ldm/Library/Caches/Google/Chrome/Default/Cache/Cache_Data 这里面更新太多了
- 搜索结果 full path 是乱序的
