# FS Icon (macOS Icons & Thumbnails)

`fs-icon/` provides best-effort PNG bytes for file and folder visuals on macOS.

## Public API
- `icon_of_path(path)` -> Quick Look first, then NSWorkspace fallback
- `icon_of_path_ns(path)` -> Finder-style icon from `NSWorkspace`
- `icon_of_path_ql(path)` -> Quick Look thumbnail for image-like files
- `image_dimension(path)` -> cheap width/height probe via Image I/O
- `scale_with_aspect_ratio(...)` -> shared size helper

## NSWorkspace path
`icon_of_path_ns(...)`:
1. asks `NSWorkspace::sharedWorkspace().iconForFile(...)`
2. prefers a representation already close to `32x32`
3. otherwise scales the image to fit within a `32x32` box
4. rasterizes through `NSBitmapImageRep`
5. returns PNG bytes

Everything runs inside an autorelease pool.

## Quick Look path
`icon_of_path_ql(...)` only proceeds if `image_dimension(...)` succeeds. That deliberately limits Quick Look thumbnails to image-like inputs.

Flow:
1. read intrinsic image size through `CGImageSource`
2. scale into a `64x64` thumbnail box
3. create `QLThumbnailGenerationRequest`
4. bridge the async completion handler back to Rust through a bounded `crossbeam_channel`
5. convert the returned `NSImage` into PNG bytes

If any step fails, the function returns `None` and callers can fall back to `icon_of_path_ns(...)`.

## Integration in Cardinal
- `get_nodes_info` uses `icon_of_path_ns(...)` for baseline row icons.
- Background viewport prefetch uses `icon_of_path_ql(...)` for richer thumbnails on visible rows.
- The UI only consumes `data:image/png;base64,...` strings; it does not know or care which native path produced the bytes.

## Practical caveats
- `icon_of_path_ql(...)` is intentionally narrow and will return `None` for many non-image files.
- The crate does not cache results on its own; Cardinal handles batching and dedup higher up.
