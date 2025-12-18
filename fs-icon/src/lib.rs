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

pub fn icon_of_path(path: &str, requested_size: f64) -> Option<Vec<u8>> {
    // Try QuickLook for images first (optimized path with aspect ratio)
    if let Some(data) = icon_of_path_ql(path, requested_size) {
        return Some(data);
    }
    // Try generic QuickLook for PDFs and other file types
    if let Some(data) = icon_of_path_ql_generic(path, requested_size) {
        return Some(data);
    }
    // Fallback to NSWorkspace icon
    icon_of_path_ns(path, requested_size)
}

// https://stackoverflow.com/questions/73062803/resizing-nsimage-keeping-aspect-ratio-reducing-the-image-size-while-trying-to-sc
pub fn icon_of_path_ns(path: &str, requested_size: f64) -> Option<Vec<u8>> {
    objc2::rc::autoreleasepool(|_| -> Option<Vec<u8>> {
        let path_ns = NSString::from_str(path);
        let image = NSWorkspace::sharedWorkspace().iconForFile(&path_ns);

        let png_data: Retained<NSData> = (|| -> Option<_> {
            unsafe {
                // https://stackoverflow.com/questions/66270656/macos-determine-real-size-of-icon-returned-from-iconforfile-method
                for image in image.representations().iter() {
                    let size = image.size();
                    if size.width > (requested_size - 1.0)
                        && size.height > (requested_size - 1.0)
                        && size.width < (requested_size + 1.0)
                        && size.height < (requested_size + 1.0)
                    {
                        // println!("representation: {}x{}", size.width, size.height);
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
            // zoom in and you will see that the small icon in Finder is 32x32, here we keep it at 64x64 for better visibility
            let (new_width, new_height) = {
                let width = requested_size;
                let height = requested_size;
                // keep aspect ratio
                let old_width = image.size().width;
                let old_height = image.size().height;
                scale_with_aspect_ratio(old_width, old_height, width, height)
            };
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

/// Internal helper to generate QuickLook thumbnails
fn generate_ql_thumbnail(
    path: &str,
    width: f64,
    height: f64,
    representation_type: QLThumbnailGenerationRequestRepresentationTypes,
) -> Option<Vec<u8>> {
    objc2::rc::autoreleasepool(|_| -> Option<Vec<u8>> {
        const THUMBNAIL_SCALE: f64 = 1.0;
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
                        representation_type,
                    );
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

/// Generate QuickLook thumbnail for any file type (PDFs, documents, etc.)
/// This function doesn't require image dimensions and works for all QuickLook-supported formats
pub fn icon_of_path_ql_generic(path: &str, requested_size: f64) -> Option<Vec<u8>> {
    generate_ql_thumbnail(
        path,
        requested_size,
        requested_size,
        QLThumbnailGenerationRequestRepresentationTypes::Thumbnail,
    )
}

pub fn icon_of_path_ql(path: &str, requested_size: f64) -> Option<Vec<u8>> {
    // We only get QLThumbnail for image, get NSWorkspace icon for other file types.
    // Therefore we just error out when image_dimension is not found.
    let (width, height) = image_dimension(path)?;
    let (width, height) = scale_with_aspect_ratio(width, height, requested_size, requested_size);
    generate_ql_thumbnail(
        path,
        width,
        height,
        QLThumbnailGenerationRequestRepresentationTypes::LowQualityThumbnail,
    )
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
        let data = icon_of_path_ns(&pwd, 32.0).unwrap();
        std::fs::write("/tmp/icon.png", data).unwrap();
    }

    #[test]
    fn test_icon_of_path_ql_normal() {
        let data = icon_of_path_ql("../cardinal/mac-icon_1024x1024.png", 64.0).unwrap();
        std::fs::write("/tmp/icon_ql.png", data).unwrap();
    }

    #[test]
    #[should_panic = "should fail for non-image file"]
    fn test_icon_of_path_ql_non_image() {
        let pwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        icon_of_path_ql(&pwd, 64.0).expect("should fail for non-image file");
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
                let _data = icon_of_path_ns(&pwd, 64.0).unwrap();
                let _data = icon_of_path_ql(&pwd, 64.0).unwrap();
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
            let icon_ns = icon_of_path_ns(&path_str, 64.0).expect("NSWorkspace icon lookup failed");
            let ns_elapsed = start_ns.elapsed();

            let start_ql = Instant::now();
            let Some(icon_ql) = icon_of_path_ql(&path_str, 64.0) else {
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
