use block2::RcBlock;
use crossbeam_channel::bounded;
use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage, NSWorkspace};
use objc2_core_foundation::{CFNumber, CFString, CFURL, Type};
use objc2_foundation::{NSData, NSDictionary, NSError, NSSize, NSString, NSURL};
use objc2_image_io::{CGImageSource, kCGImagePropertyPixelHeight, kCGImagePropertyPixelWidth};
use objc2_quick_look_thumbnailing::{
    QLThumbnailGenerationRequest, QLThumbnailGenerationRequestRepresentationTypes,
    QLThumbnailGenerator, QLThumbnailRepresentation,
};
use std::ffi::c_void;

pub fn scale_with_aspect_ratio(
    width: f64,
    height: f64,
    max_width: f64,
    max_height: f64,
) -> (f64, f64) {
    let ratio_x = max_width / width;
    let ratio_y = max_height / height;
    let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };
    (width * ratio, height * ratio)
}

pub fn icon_of_path(path: &str) -> Option<Vec<u8>> {
    if let Some(data) = icon_of_path_ql(path) {
        return Some(data);
    }
    icon_of_path_ns(path)
}

/// Extensions whose preview is a *cover image* rather than an app icon: images and videos.
/// Everything else (`.html`, `.md`, `.psd`, ...) should show the icon of whichever app the
/// system associates with that type, which is exactly what `NSWorkspace::iconForFile`
/// returns — the same artwork Finder shows, e.g. Chrome's page icon for `.html` when Chrome
/// is the default browser, or VS Code's icon once it is set as the default opener.
const IMAGE_EXTENSIONS: &[&str] = &[
    "ani", "apng", "avif", "avifs", "bmp", "bpg", "cur", "dds", "gif", "heic", "heics", "heif",
    "heifs", "hif", "ico", "jfi", "jfif", "jif", "jpe", "jpeg", "jpg", "jxl", "jxr", "pcx", "png",
    "psb", "psd", "svg", "tga", "tif", "tiff", "wdp", "webp", "wmf",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "3g2", "3gp", "3gp2", "3gpp", "amv", "asf", "asx", "avi", "bdmv", "bik", "d2v", "divx", "drc",
    "dsa", "dsm", "dss", "dsv", "evo", "f4v", "flc", "fli", "flic", "flv", "hdmov", "ifo", "ivf",
    "m1v", "m2p", "m2t", "m2ts", "m2v", "m4v", "mkv", "mov", "mp2v", "mp4", "mp4v", "mpe", "mpeg",
    "mpg", "mpls", "mpv2", "mpv4", "ogm", "ogv", "ogx", "pss", "pva", "qt", "ram", "ratdvd", "rm",
    "rmm", "rmvb", "roq", "rpm", "smil", "smk", "swf", "tod", "tp", "tpr", "tts", "uvu", "vob",
    "vp6", "webm", "wm", "wmp", "wmv", "wmx", "wvx",
];

fn extension_of(path: &str) -> Option<String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let pos = name.rfind('.')?;
    if pos + 1 >= name.len() {
        return None;
    }
    Some(name[pos + 1..].to_ascii_lowercase())
}

/// True when the file should be represented by a Quick Look cover image (its own content)
/// instead of the icon of the app that opens it.
pub fn prefers_cover_thumbnail(path: &str) -> bool {
    match extension_of(path) {
        Some(ext) => {
            IMAGE_EXTENSIONS.contains(&ext.as_str()) || VIDEO_EXTENSIONS.contains(&ext.as_str())
        }
        None => false,
    }
}

/// Cache of "icon of the app that opens this extension" bitmaps, keyed by
/// `extension@size`. A directory of 500 `.html` files then costs one LaunchServices lookup
/// instead of 500, which is the bulk of the per-row icon cost.
static APP_ICON_CACHE: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, Option<Vec<u8>>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

fn cached_app_icon(path: &str, max_size: f64) -> Option<Vec<u8>> {
    let key = match extension_of(path) {
        Some(ext) => format!("{ext}@{}", max_size.round() as i64),
        // No extension (or a directory): key on "directory" so folder icons share one entry.
        None => format!("dir@{}", max_size.round() as i64),
    };

    if let Some(hit) = APP_ICON_CACHE
        .lock()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
    {
        return hit;
    }

    let rendered = icon_of_path_ns_sized(path, max_size);
    if let Ok(mut cache) = APP_ICON_CACHE.lock() {
        // Only cache successes; a transient failure should be retried.
        if rendered.is_some() {
            cache.insert(key, rendered.clone());
        }
    }
    rendered
}

/// Size-aware icon used by the async icon pipeline.
///
/// Images and videos get Quick Look cover art (their own content, like Finder's preview);
/// every other path — including directories — gets the system/Finder icon, which follows the
/// user's default-application association for that file type.
pub fn icon_of_path_sized(path: &str, max_size: f64) -> Option<Vec<u8>> {
    if prefers_cover_thumbnail(path)
        && let Some(data) = icon_of_path_ql_sized(path, max_size)
    {
        return Some(data);
    }
    cached_app_icon(path, max_size)
}

/// Size used by the results list, where icons are drawn at 16 CSS px.
pub const DEFAULT_ICON_SIZE: f64 = 32.0;
/// Largest raster the tiled view needs: the "extra large" preset (192 CSS px) at 2x scale.
pub const LARGE_ICON_SIZE: f64 = 384.0;

// https://stackoverflow.com/questions/73062803/resizing-nsimage-keeping-aspect-ratio-reducing-the-image-size-while-trying-to-sc
pub fn icon_of_path_ns(path: &str) -> Option<Vec<u8>> {
    icon_of_path_ns_sized(path, DEFAULT_ICON_SIZE)
}

/// Rasterises the Finder icon for `path`, fitting the longest edge into `max_size` points.
/// Requires a larger `max_size` than the default to be usable in the tiled view.
pub fn icon_of_path_ns_sized(path: &str, max_size: f64) -> Option<Vec<u8>> {
    objc2::rc::autoreleasepool(|_| -> Option<Vec<u8>> {
        let path_ns = NSString::from_str(path);
        let image = NSWorkspace::sharedWorkspace().iconForFile(&path_ns);

        let png_data: Retained<NSData> = (|| -> Option<_> {
            unsafe {
                // https://stackoverflow.com/questions/66270656/macos-determine-real-size-of-icon-returned-from-iconforfile-method
                // Reuse an exactly-sized representation when one exists; it avoids relying on
                // NSImage's own interpolation and keeps the list view pixel-identical.
                if (max_size - DEFAULT_ICON_SIZE).abs() < f64::EPSILON {
                    for image in image.representations().iter() {
                        let size = image.size();
                        if size.width > 31.0
                            && size.height > 31.0
                            && size.width < 33.0
                            && size.height < 33.0
                        {
                            let new_image = NSImage::imageWithSize_flipped_drawingHandler(
                                NSSize::new(size.width, size.height),
                                false,
                                &block2::RcBlock::new(move |rect| {
                                    image.drawInRect(rect);
                                    true.into()
                                }),
                            );
                            return NSBitmapImageRep::imageRepWithData(
                                &*new_image.TIFFRepresentation()?,
                            )?
                            .representationUsingType_properties(
                                NSBitmapImageFileType::PNG,
                                &NSDictionary::new(),
                            );
                        }
                    }
                }
            }
            // Draw at the requested size; NSImage picks the best backing representation.
            let (new_width, new_height) = scale_with_aspect_ratio(
                image.size().width,
                image.size().height,
                max_size,
                max_size,
            );
            unsafe {
                let new_image = NSImage::imageWithSize_flipped_drawingHandler(
                    NSSize::new(new_width, new_height),
                    false,
                    &block2::RcBlock::new(move |rect| {
                        image.drawInRect(rect);
                        true.into()
                    }),
                );
                NSBitmapImageRep::imageRepWithData(&*new_image.TIFFRepresentation()?)?
                    .representationUsingType_properties(
                        NSBitmapImageFileType::PNG,
                        &NSDictionary::new(),
                    )
            }
        })()?;
        Some(png_data.to_vec())
    })
}

pub fn image_dimension(image_path: &str) -> Option<(f64, f64)> {
    // https://stackoverflow.com/questions/6468747/get-image-width-and-height-before-loading-it-completely-in-iphone
    objc2::rc::autoreleasepool(|_| -> Option<(f64, f64)> {
        let path_cf_url = CFURL::from_file_path(image_path)?;
        unsafe {
            let image_source = CGImageSource::with_url(&path_cf_url, None)?;
            let image_header = image_source.properties_at_index(0, None)?;
            let width = {
                let width = image_header
                    .value(kCGImagePropertyPixelWidth as *const CFString as *const c_void);
                CFNumber::retain(width.cast::<CFNumber>().as_ref()?)
            };
            let height = {
                let height = image_header
                    .value(kCGImagePropertyPixelHeight as *const CFString as *const c_void);
                CFNumber::retain(height.cast::<CFNumber>().as_ref()?)
            };
            Some((width.as_f64()?, height.as_f64()?))
        }
    })
}

pub fn icon_of_path_ql(path: &str) -> Option<Vec<u8>> {
    icon_of_path_ql_sized(path, 256.0)
}

/// Quick Look cover art (image preview, video frame, ...) fitted into `max_size` points.
pub fn icon_of_path_ql_sized(path: &str, max_size: f64) -> Option<Vec<u8>> {
    objc2::rc::autoreleasepool(|_| -> Option<Vec<u8>> {
        let thumbnail_size = if max_size.is_finite() && max_size > 0.0 {
            max_size
        } else {
            256.0
        };
        const THUMBNAIL_SCALE: f64 = 1.0;
        // Prefer the file's intrinsic pixel size to preserve its aspect ratio when we
        // can probe it (images). For videos and other documents image_dimension returns
        // nothing, so fall back to a large square target; Quick Look will still produce
        // a cover frame / preview for those.
        let (width, height) = image_dimension(path)
            .map(|(w, h)| scale_with_aspect_ratio(w, h, thumbnail_size, thumbnail_size))
            .unwrap_or((thumbnail_size, thumbnail_size));
        let path_url = NSURL::fileURLWithPath(&NSString::from_str(path));
        let generator = unsafe { QLThumbnailGenerator::sharedGenerator() };
        {
            let (tx, rx) = bounded(1);
            unsafe {
                let request =
                    QLThumbnailGenerationRequest::initWithFileAtURL_size_scale_representationTypes(
                        QLThumbnailGenerationRequest::alloc(),
                        &path_url,
                        NSSize::new(width, height),
                        THUMBNAIL_SCALE,
                        QLThumbnailGenerationRequestRepresentationTypes::LowQualityThumbnail
                            | QLThumbnailGenerationRequestRepresentationTypes::Thumbnail,
                    );
                // Icon mode decorates the thumbnail with a file-like frame/shadow; disable it
                // so we get the raw cover art (video frame, image preview) like Finder shows.
                request.setIconMode(false);
                generator.generateBestRepresentationForRequest_completionHandler(
                    &request,
                    &RcBlock::new(
                        move |result: *mut QLThumbnailRepresentation, _error: *mut NSError| {
                            let _ = tx.send(result.as_ref().and_then(|result| {
                                Some(
                                    NSBitmapImageRep::imageRepWithData(
                                        &*result.NSImage().TIFFRepresentation()?,
                                    )?
                                    .representationUsingType_properties(
                                        NSBitmapImageFileType::PNG,
                                        &NSDictionary::new(),
                                    )?
                                    .to_vec(),
                                )
                            }));
                        },
                    ),
                )
            };
            rx.recv().ok().flatten()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_icon_of_path_normal() {
        let pwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let data = icon_of_path_ns(&pwd).unwrap();
        std::fs::write("/tmp/icon.png", data).unwrap();
    }

    #[test]
    fn classifies_cover_thumbnail_extensions() {
        // Images and videos show their own content, like Finder previews.
        assert!(prefers_cover_thumbnail("/tmp/clip.mp4"));
        assert!(prefers_cover_thumbnail("/tmp/photo.JPG"));
        assert!(prefers_cover_thumbnail("/tmp/anim.webp"));

        // Documents and code show the icon of their default app instead.
        assert!(!prefers_cover_thumbnail("/tmp/page.html"));
        assert!(!prefers_cover_thumbnail("/tmp/notes.md"));
        assert!(!prefers_cover_thumbnail("/tmp/data.json"));
        assert!(!prefers_cover_thumbnail("/tmp/no-extension"));
        assert!(!prefers_cover_thumbnail("/tmp/some.directory/"));
    }

    #[test]
    fn caches_app_icons_per_extension() {
        let pwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let first = cached_app_icon(&pwd, DEFAULT_ICON_SIZE);
        let second = cached_app_icon(&pwd, DEFAULT_ICON_SIZE);

        assert!(first.is_some());
        assert_eq!(first, second);
    }

    #[test]
    fn test_icon_of_path_ns_sized_larger_than_default() {
        // Grid presets draw icons up to 192 CSS px, so the tiling view needs a bigger
        // raster than the 32px list icon; a 32px bitmap cannot be scaled up cleanly.
        let pwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let small = icon_of_path_ns_sized(&pwd, DEFAULT_ICON_SIZE).unwrap();
        let large = icon_of_path_ns_sized(&pwd, LARGE_ICON_SIZE).unwrap();
        assert!(!small.is_empty());
        assert!(!large.is_empty());
        assert!(
            large.len() > small.len(),
            "large icon ({} bytes) should carry more pixels than the default ({} bytes)",
            large.len(),
            small.len()
        );
    }

    #[test]
    fn test_icon_of_path_ql_normal() {
        let data = icon_of_path_ql("../cardinal/mac-icon_1024x1024.png").unwrap();
        std::fs::write("/tmp/icon_ql.png", data).unwrap();
    }

    #[test]
    fn test_icon_of_path_ql_non_image() {
        // Quick Look thumbnails are no longer gated on image_dimension so non-image
        // inputs (e.g. videos, documents) can also produce a cover art preview. The
        // call must simply never panic; the result (Some/None) is provider-dependent.
        let pwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let _ = icon_of_path_ql(&pwd);
    }

    #[test]
    fn test_icon_dimension() {
        let (width, height) = image_dimension("../cardinal/mac-icon_1024x1024.png").unwrap();
        assert_eq!(width, 1024.0);
        assert_eq!(height, 1024.0);
    }

    #[test]
    fn test_icon_dimension_not_available_for_non_image() {
        let pwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(image_dimension(&pwd).is_none());
    }

    #[test]
    fn test_scale_with_aspect_ratio() {
        // Scales down square
        assert_eq!(
            scale_with_aspect_ratio(100.0, 100.0, 50.0, 50.0),
            (50.0, 50.0)
        );
        // Scales up square
        assert_eq!(
            scale_with_aspect_ratio(50.0, 50.0, 100.0, 100.0),
            (100.0, 100.0)
        );
        // Wide scales down
        assert_eq!(
            scale_with_aspect_ratio(200.0, 100.0, 50.0, 50.0),
            (50.0, 25.0)
        );
        // Tall scales down
        assert_eq!(
            scale_with_aspect_ratio(100.0, 200.0, 50.0, 50.0),
            (25.0, 50.0)
        );
    }

    #[test]
    #[ignore]
    fn test_icon_of_file_leak() {
        let pwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        loop {
            for _ in 0..10000 {
                let _data = icon_of_path_ns(&pwd).unwrap();
                let _data = icon_of_path_ql(&pwd).unwrap();
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    #[test]
    #[ignore = "local speed test"]
    fn test_icon_generation_for_qq_images() {
        let dir = "/Users/0ldm/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_f0d0b80219b392fd75eeb26a6a67027c/nt_data/Pic/2025-06/Ori/";
        let entries = std::fs::read_dir(dir).expect("failed to read QQ image directory");

        let mut ns_total: Duration = Duration::default();
        let mut ql_total: Duration = Duration::default();
        let mut processed = 0usize;

        for entry in entries {
            let entry = entry.expect("failed to read entry");
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let path_str = path.to_string_lossy().into_owned();

            let start_ns = Instant::now();
            let icon_ns = icon_of_path_ns(&path_str).expect("NSWorkspace icon lookup failed");
            let ns_elapsed = start_ns.elapsed();

            let start_ql = Instant::now();
            let Some(icon_ql) = icon_of_path_ql(&path_str) else {
                println!("QuickLook thumbnail generation failed for path {path_str}");
                continue;
            };
            let ql_elapsed = start_ql.elapsed();

            ns_total += ns_elapsed;
            ql_total += ql_elapsed;

            println!(
                "{} -> ns: {:?}, ql: {:?}, ns_bytes: {}, ql_bytes: {}",
                path.display(),
                ns_elapsed,
                ql_elapsed,
                icon_ns.len(),
                icon_ql.len()
            );

            processed += 1;
        }

        println!(
            "Processed {processed} files with total durations ns={ns_total:?}, ql={ql_total:?}"
        );

        assert!(processed > 0, "no image files were processed");
    }
}
