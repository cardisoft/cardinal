use super::prelude::*;

#[test]
fn test_type_and_macro_filters() {
    let tmp = TempDir::new("query_type_filters").unwrap();
    fs::write(tmp.path().join("photo.png"), b"x").unwrap();
    fs::write(tmp.path().join("song.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let pictures = cache.search("type:picture").unwrap();
    assert_eq!(pictures.len(), 1);
    let path = cache.node_path(*pictures.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("photo.png")));

    let audio = cache.search("audio:").unwrap();
    assert_eq!(audio.len(), 1);
    let song = cache.node_path(*audio.first().unwrap()).unwrap();
    assert!(song.ends_with(PathBuf::from("song.mp3")));

    let documents = cache.search("doc:").unwrap();
    assert_eq!(documents.len(), 1);
    let doc_path = cache.node_path(*documents.first().unwrap()).unwrap();
    assert!(doc_path.ends_with(PathBuf::from("notes.txt")));
}

#[test]
fn test_audio_macro_with_argument_behaves_like_and() {
    let tmp = TempDir::new("query_audio_argument").unwrap();
    fs::write(tmp.path().join("song_beats.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("song_other.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("audio:beats").unwrap();
    assert_eq!(results.len(), 1);
    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("song_beats.mp3")));
}

#[test]
fn test_type_filter_respects_parent_base() {
    let tmp = TempDir::new("type_filter_base").unwrap();
    fs::create_dir(tmp.path().join("media")).unwrap();
    fs::write(tmp.path().join("media/keep.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("skip.jpg"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let keep_idx = cache.search("keep.jpg").unwrap()[0];
    let skip_idx = cache.search("skip.jpg").unwrap()[0];
    assert!(cache.file_nodes[skip_idx].metadata.is_none());

    let results = cache
        .search(&format!(
            "parent:{} type:picture",
            tmp.path().join("media").display()
        ))
        .unwrap();
    assert_eq!(results.len(), 1);
    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("keep.jpg")));

    assert!(
        cache.file_nodes[skip_idx].metadata.is_none(),
        "type filter should not touch nodes outside the parent base"
    );
    assert!(
        cache.file_nodes[keep_idx].metadata.is_none(),
        "type filter should not touch nodes inside the parent base"
    );
}

#[test]
fn test_type_picture_comprehensive() {
    let tmp = TempDir::new("type_picture_comp").unwrap();
    // Common image formats
    fs::write(tmp.path().join("photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("image.jpeg"), b"x").unwrap();
    fs::write(tmp.path().join("graphic.png"), b"x").unwrap();
    fs::write(tmp.path().join("animation.gif"), b"x").unwrap();
    fs::write(tmp.path().join("bitmap.bmp"), b"x").unwrap();
    fs::write(tmp.path().join("texture.tif"), b"x").unwrap();
    fs::write(tmp.path().join("scan.tiff"), b"x").unwrap();
    fs::write(tmp.path().join("web.webp"), b"x").unwrap();
    fs::write(tmp.path().join("icon.ico"), b"x").unwrap();
    fs::write(tmp.path().join("vector.svg"), b"x").unwrap();
    // iPhone formats
    fs::write(tmp.path().join("iphone.heic"), b"x").unwrap();
    fs::write(tmp.path().join("burst.heif"), b"x").unwrap();
    // RAW formats
    fs::write(tmp.path().join("sony.arw"), b"x").unwrap();
    fs::write(tmp.path().join("canon.cr2"), b"x").unwrap();
    fs::write(tmp.path().join("olympus.orf"), b"x").unwrap();
    fs::write(tmp.path().join("fuji.raf"), b"x").unwrap();
    // Professional formats
    fs::write(tmp.path().join("layer.psd"), b"x").unwrap();
    fs::write(tmp.path().join("design.ai"), b"x").unwrap();
    // Non-picture files
    fs::write(tmp.path().join("document.txt"), b"x").unwrap();
    fs::write(tmp.path().join("video.mp4"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let pictures = cache.search("type:picture").unwrap();
    assert_eq!(pictures.len(), 18, "Should match all 18 image formats");

    // Test alternate names
    let pictures_alt = cache.search("type:pictures").unwrap();
    assert_eq!(pictures_alt.len(), 18);

    let images = cache.search("type:image").unwrap();
    assert_eq!(images.len(), 18);

    let photos = cache.search("type:photo").unwrap();
    assert_eq!(photos.len(), 18);

    // Test case insensitivity
    let upper = cache.search("type:PICTURE").unwrap();
    assert_eq!(upper.len(), 18);

    let mixed = cache.search("type:PiCtUrE").unwrap();
    assert_eq!(mixed.len(), 18);
}

#[test]
fn test_type_video_comprehensive() {
    let tmp = TempDir::new("type_video_comp").unwrap();
    fs::write(tmp.path().join("clip.mp4"), b"x").unwrap();
    fs::write(tmp.path().join("iphone.m4v"), b"x").unwrap();
    fs::write(tmp.path().join("quicktime.mov"), b"x").unwrap();
    fs::write(tmp.path().join("windows.avi"), b"x").unwrap();
    fs::write(tmp.path().join("mkv_file.mkv"), b"x").unwrap();
    fs::write(tmp.path().join("wm_video.wmv"), b"x").unwrap();
    fs::write(tmp.path().join("web.webm"), b"x").unwrap();
    fs::write(tmp.path().join("flash.flv"), b"x").unwrap();
    fs::write(tmp.path().join("mpeg1.mpg"), b"x").unwrap();
    fs::write(tmp.path().join("mpeg2.mpeg"), b"x").unwrap();
    fs::write(tmp.path().join("mobile.3gp"), b"x").unwrap();
    fs::write(tmp.path().join("mobile2.3g2"), b"x").unwrap();
    fs::write(tmp.path().join("transport.ts"), b"x").unwrap();
    fs::write(tmp.path().join("avchd.mts"), b"x").unwrap();
    fs::write(tmp.path().join("bluray.m2ts"), b"x").unwrap();
    // Non-video files
    fs::write(tmp.path().join("audio.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("doc.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let videos = cache.search("type:video").unwrap();
    assert_eq!(videos.len(), 15);

    let videos_alt = cache.search("type:videos").unwrap();
    assert_eq!(videos_alt.len(), 15);

    let movies = cache.search("type:movie").unwrap();
    assert_eq!(movies.len(), 15);

    let movies_alt = cache.search("type:movies").unwrap();
    assert_eq!(movies_alt.len(), 15);
}

#[test]
fn test_type_audio_comprehensive() {
    let tmp = TempDir::new("type_audio_comp").unwrap();
    fs::write(tmp.path().join("song.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("wave.wav"), b"x").unwrap();
    fs::write(tmp.path().join("lossless.flac"), b"x").unwrap();
    fs::write(tmp.path().join("compressed.aac"), b"x").unwrap();
    fs::write(tmp.path().join("vorbis.ogg"), b"x").unwrap();
    fs::write(tmp.path().join("vorbis2.oga"), b"x").unwrap();
    fs::write(tmp.path().join("modern.opus"), b"x").unwrap();
    fs::write(tmp.path().join("windows.wma"), b"x").unwrap();
    fs::write(tmp.path().join("apple.m4a"), b"x").unwrap();
    fs::write(tmp.path().join("apple_lossless.alac"), b"x").unwrap();
    fs::write(tmp.path().join("uncompressed.aiff"), b"x").unwrap();
    // Non-audio files
    fs::write(tmp.path().join("video.mp4"), b"x").unwrap();
    fs::write(tmp.path().join("text.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let audio = cache.search("type:audio").unwrap();
    assert_eq!(audio.len(), 11);

    let audio_alt = cache.search("type:audios").unwrap();
    assert_eq!(audio_alt.len(), 11);

    let music = cache.search("type:music").unwrap();
    assert_eq!(music.len(), 11);

    let songs = cache.search("type:song").unwrap();
    assert_eq!(songs.len(), 11);

    let songs_alt = cache.search("type:songs").unwrap();
    assert_eq!(songs_alt.len(), 11);
}

#[test]
fn test_type_document_comprehensive() {
    let tmp = TempDir::new("type_doc_comp").unwrap();
    fs::write(tmp.path().join("plain.txt"), b"x").unwrap();
    fs::write(tmp.path().join("markdown.md"), b"x").unwrap();
    fs::write(tmp.path().join("restructured.rst"), b"x").unwrap();
    fs::write(tmp.path().join("word_old.doc"), b"x").unwrap();
    fs::write(tmp.path().join("word_new.docx"), b"x").unwrap();
    fs::write(tmp.path().join("rich.rtf"), b"x").unwrap();
    fs::write(tmp.path().join("opendoc.odt"), b"x").unwrap();
    fs::write(tmp.path().join("portable.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("apple.pages"), b"x").unwrap();
    fs::write(tmp.path().join("apple_rtf.rtfd"), b"x").unwrap();
    // Non-document files
    fs::write(tmp.path().join("image.png"), b"x").unwrap();
    fs::write(tmp.path().join("code.rs"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let docs = cache.search("type:doc").unwrap();
    assert_eq!(docs.len(), 10);

    let docs_alt = cache.search("type:docs").unwrap();
    assert_eq!(docs_alt.len(), 10);

    let documents = cache.search("type:document").unwrap();
    assert_eq!(documents.len(), 10);

    let documents_alt = cache.search("type:documents").unwrap();
    assert_eq!(documents_alt.len(), 10);

    let text = cache.search("type:text").unwrap();
    assert_eq!(text.len(), 10);

    let office = cache.search("type:office").unwrap();
    assert_eq!(office.len(), 10);
}

#[test]
fn test_type_presentation_comprehensive() {
    let tmp = TempDir::new("type_presentation").unwrap();
    fs::write(tmp.path().join("powerpoint_old.ppt"), b"x").unwrap();
    fs::write(tmp.path().join("powerpoint_new.pptx"), b"x").unwrap();
    fs::write(tmp.path().join("apple.key"), b"x").unwrap();
    fs::write(tmp.path().join("opendoc.odp"), b"x").unwrap();
    fs::write(tmp.path().join("document.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let pres = cache.search("type:presentation").unwrap();
    assert_eq!(pres.len(), 4);

    let pres_alt = cache.search("type:presentations").unwrap();
    assert_eq!(pres_alt.len(), 4);

    let ppt = cache.search("type:ppt").unwrap();
    assert_eq!(ppt.len(), 4);

    let slides = cache.search("type:slides").unwrap();
    assert_eq!(slides.len(), 4);
}

#[test]
fn test_type_spreadsheet_comprehensive() {
    let tmp = TempDir::new("type_spreadsheet").unwrap();
    fs::write(tmp.path().join("excel_old.xls"), b"x").unwrap();
    fs::write(tmp.path().join("excel_new.xlsx"), b"x").unwrap();
    fs::write(tmp.path().join("data.csv"), b"x").unwrap();
    fs::write(tmp.path().join("apple.numbers"), b"x").unwrap();
    fs::write(tmp.path().join("opendoc.ods"), b"x").unwrap();
    fs::write(tmp.path().join("text.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let sheets = cache.search("type:spreadsheet").unwrap();
    assert_eq!(sheets.len(), 5);

    let sheets_alt = cache.search("type:spreadsheets").unwrap();
    assert_eq!(sheets_alt.len(), 5);

    let xls = cache.search("type:xls").unwrap();
    assert_eq!(xls.len(), 5);

    let excel = cache.search("type:excel").unwrap();
    assert_eq!(excel.len(), 5);

    let sheet = cache.search("type:sheet").unwrap();
    assert_eq!(sheet.len(), 5);

    let sheets2 = cache.search("type:sheets").unwrap();
    assert_eq!(sheets2.len(), 5);
}

#[test]
fn test_type_pdf_filter() {
    let tmp = TempDir::new("type_pdf").unwrap();
    fs::write(tmp.path().join("manual.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("report.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("guide.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("doc.docx"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let pdfs = cache.search("type:pdf").unwrap();
    assert_eq!(pdfs.len(), 3);
}

#[test]
fn test_type_archive_comprehensive() {
    let tmp = TempDir::new("type_archive").unwrap();
    fs::write(tmp.path().join("archive.zip"), b"x").unwrap();
    fs::write(tmp.path().join("winrar.rar"), b"x").unwrap();
    fs::write(tmp.path().join("seven.7z"), b"x").unwrap();
    fs::write(tmp.path().join("tarball.tar"), b"x").unwrap();
    fs::write(tmp.path().join("gzip.gz"), b"x").unwrap();
    fs::write(tmp.path().join("tar_gzip.tgz"), b"x").unwrap();
    fs::write(tmp.path().join("bzip.bz2"), b"x").unwrap();
    fs::write(tmp.path().join("xz_archive.xz"), b"x").unwrap();
    fs::write(tmp.path().join("zstd.zst"), b"x").unwrap();
    fs::write(tmp.path().join("cabinet.cab"), b"x").unwrap();
    fs::write(tmp.path().join("disc.iso"), b"x").unwrap();
    fs::write(tmp.path().join("macos.dmg"), b"x").unwrap();
    fs::write(tmp.path().join("text.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let archives = cache.search("type:archive").unwrap();
    assert_eq!(archives.len(), 12);

    let archives_alt = cache.search("type:archives").unwrap();
    assert_eq!(archives_alt.len(), 12);

    let compressed = cache.search("type:compressed").unwrap();
    assert_eq!(compressed.len(), 12);

    let zip = cache.search("type:zip").unwrap();
    assert_eq!(zip.len(), 12);
}

#[test]
fn test_type_code_comprehensive() {
    let tmp = TempDir::new("type_code").unwrap();
    // Rust
    fs::write(tmp.path().join("main.rs"), b"x").unwrap();
    // TypeScript/JavaScript
    fs::write(tmp.path().join("app.ts"), b"x").unwrap();
    fs::write(tmp.path().join("component.tsx"), b"x").unwrap();
    fs::write(tmp.path().join("script.js"), b"x").unwrap();
    fs::write(tmp.path().join("view.jsx"), b"x").unwrap();
    // C/C++
    fs::write(tmp.path().join("program.c"), b"x").unwrap();
    fs::write(tmp.path().join("impl.cc"), b"x").unwrap();
    fs::write(tmp.path().join("source.cpp"), b"x").unwrap();
    fs::write(tmp.path().join("alt.cxx"), b"x").unwrap();
    fs::write(tmp.path().join("header.h"), b"x").unwrap();
    fs::write(tmp.path().join("header2.hpp"), b"x").unwrap();
    fs::write(tmp.path().join("header3.hh"), b"x").unwrap();
    // Other languages
    fs::write(tmp.path().join("Main.java"), b"x").unwrap();
    fs::write(tmp.path().join("Program.cs"), b"x").unwrap();
    fs::write(tmp.path().join("script.py"), b"x").unwrap();
    fs::write(tmp.path().join("server.go"), b"x").unwrap();
    fs::write(tmp.path().join("app.rb"), b"x").unwrap();
    fs::write(tmp.path().join("ViewController.swift"), b"x").unwrap();
    fs::write(tmp.path().join("MainActivity.kt"), b"x").unwrap();
    fs::write(tmp.path().join("Script.kts"), b"x").unwrap();
    fs::write(tmp.path().join("index.php"), b"x").unwrap();
    // Web
    fs::write(tmp.path().join("page.html"), b"x").unwrap();
    fs::write(tmp.path().join("style.css"), b"x").unwrap();
    fs::write(tmp.path().join("vars.scss"), b"x").unwrap();
    fs::write(tmp.path().join("mixins.sass"), b"x").unwrap();
    fs::write(tmp.path().join("theme.less"), b"x").unwrap();
    // Config
    fs::write(tmp.path().join("config.json"), b"x").unwrap();
    fs::write(tmp.path().join("settings.yaml"), b"x").unwrap();
    fs::write(tmp.path().join("docker.yml"), b"x").unwrap();
    fs::write(tmp.path().join("Cargo.toml"), b"x").unwrap();
    fs::write(tmp.path().join("setup.ini"), b"x").unwrap();
    fs::write(tmp.path().join("app.cfg"), b"x").unwrap();
    // Shell scripts
    fs::write(tmp.path().join("build.sh"), b"x").unwrap();
    fs::write(tmp.path().join("setup.zsh"), b"x").unwrap();
    fs::write(tmp.path().join("config.fish"), b"x").unwrap();
    fs::write(tmp.path().join("script.ps1"), b"x").unwrap();
    fs::write(tmp.path().join("module.psm1"), b"x").unwrap();
    // Database
    fs::write(tmp.path().join("schema.sql"), b"x").unwrap();
    // Other scripting
    fs::write(tmp.path().join("game.lua"), b"x").unwrap();
    fs::write(tmp.path().join("script.pl"), b"x").unwrap();
    fs::write(tmp.path().join("module.pm"), b"x").unwrap();
    fs::write(tmp.path().join("analysis.r"), b"x").unwrap();
    fs::write(tmp.path().join("main.m"), b"x").unwrap();
    fs::write(tmp.path().join("bridge.mm"), b"x").unwrap();
    fs::write(tmp.path().join("app.dart"), b"x").unwrap();
    fs::write(tmp.path().join("service.scala"), b"x").unwrap();
    fs::write(tmp.path().join("phoenix.ex"), b"x").unwrap();
    fs::write(tmp.path().join("test.exs"), b"x").unwrap();
    // Non-code files
    fs::write(tmp.path().join("doc.txt"), b"x").unwrap();
    fs::write(tmp.path().join("image.png"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let code = cache.search("type:code").unwrap();
    // The test creates 47 code files + Cargo.toml = 48 total
    assert_eq!(code.len(), 48);

    let source = cache.search("type:source").unwrap();
    assert_eq!(source.len(), 48);

    let dev = cache.search("type:dev").unwrap();
    assert_eq!(dev.len(), 48);
}

#[test]
fn test_type_executable_comprehensive() {
    let tmp = TempDir::new("type_exe").unwrap();
    fs::write(tmp.path().join("program.exe"), b"x").unwrap();
    fs::write(tmp.path().join("installer.msi"), b"x").unwrap();
    fs::write(tmp.path().join("script.bat"), b"x").unwrap();
    fs::write(tmp.path().join("command.cmd"), b"x").unwrap();
    fs::write(tmp.path().join("dos.com"), b"x").unwrap();
    fs::write(tmp.path().join("powershell.ps1"), b"x").unwrap();
    fs::write(tmp.path().join("module.psm1"), b"x").unwrap();
    fs::write(tmp.path().join("Calculator.app"), b"x").unwrap();
    fs::write(tmp.path().join("mobile.apk"), b"x").unwrap();
    fs::write(tmp.path().join("ios.ipa"), b"x").unwrap();
    fs::write(tmp.path().join("java.jar"), b"x").unwrap();
    fs::write(tmp.path().join("binary.bin"), b"x").unwrap();
    fs::write(tmp.path().join("linux.run"), b"x").unwrap();
    fs::write(tmp.path().join("macos.pkg"), b"x").unwrap();
    fs::write(tmp.path().join("text.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let exe = cache.search("type:exe").unwrap();
    assert_eq!(exe.len(), 14);

    let exec = cache.search("type:exec").unwrap();
    assert_eq!(exec.len(), 14);

    let executable = cache.search("type:executable").unwrap();
    assert_eq!(executable.len(), 14);

    let executables = cache.search("type:executables").unwrap();
    assert_eq!(executables.len(), 14);

    let program = cache.search("type:program").unwrap();
    assert_eq!(program.len(), 14);

    let programs = cache.search("type:programs").unwrap();
    assert_eq!(programs.len(), 14);

    let app = cache.search("type:app").unwrap();
    assert_eq!(app.len(), 14);

    let apps = cache.search("type:apps").unwrap();
    assert_eq!(apps.len(), 14);
}

#[test]
fn test_type_file_folder_filters() {
    let tmp = TempDir::new("type_file_folder").unwrap();
    fs::write(tmp.path().join("file1.txt"), b"x").unwrap();
    fs::write(tmp.path().join("file2.md"), b"x").unwrap();
    fs::create_dir(tmp.path().join("folder1")).unwrap();
    fs::create_dir(tmp.path().join("folder2")).unwrap();
    fs::write(tmp.path().join("folder1/nested.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let files = cache.search("type:file").unwrap();
    assert_eq!(files.len(), 3, "Should match only files");

    let files_alt = cache.search("type:files").unwrap();
    assert_eq!(files_alt.len(), 3);

    let folders = cache.search("type:folder").unwrap();
    // Should match folder1, folder2, and the root directory
    assert_eq!(folders.len(), 3);

    let folders_alt = cache.search("type:folders").unwrap();
    assert_eq!(folders_alt.len(), 3);

    let dirs = cache.search("type:dir").unwrap();
    assert_eq!(dirs.len(), 3);

    let directory = cache.search("type:directory").unwrap();
    assert_eq!(directory.len(), 3);
}

#[test]
fn test_type_filter_unknown_category_error() {
    let tmp = TempDir::new("type_unknown").unwrap();
    fs::write(tmp.path().join("file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let result = cache.search("type:unknowncategory");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unknown type category")
    );
}

#[test]
fn test_type_filter_empty_argument_error() {
    let tmp = TempDir::new("type_empty").unwrap();
    fs::write(tmp.path().join("file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let result = cache.search("type:");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("requires a category")
    );
}

#[test]
fn test_audio_macro_no_arguments() {
    let tmp = TempDir::new("audio_macro").unwrap();
    fs::write(tmp.path().join("song.mp3"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let result = cache.search("audio:");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_video_macro_no_arguments() {
    let tmp = TempDir::new("video_macro").unwrap();
    fs::write(tmp.path().join("clip.mp4"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let result = cache.search("video:");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_doc_macro_no_arguments() {
    let tmp = TempDir::new("doc_macro").unwrap();
    fs::write(tmp.path().join("note.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let result = cache.search("doc:");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_exe_macro_no_arguments() {
    let tmp = TempDir::new("exe_macro").unwrap();
    fs::write(tmp.path().join("app.exe"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let result = cache.search("exe:");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_type_filter_combined_with_name_search() {
    let tmp = TempDir::new("type_combined").unwrap();
    fs::write(tmp.path().join("report.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("report.docx"), b"x").unwrap();
    fs::write(tmp.path().join("summary.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("image.png"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("report type:doc").unwrap();
    assert_eq!(results.len(), 2);

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("report.pdf")));
    assert!(paths.iter().any(|p| p.ends_with("report.docx")));
}

#[test]
fn test_type_filter_with_or_operator() {
    let tmp = TempDir::new("type_or").unwrap();
    fs::write(tmp.path().join("song.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("clip.mp4"), b"x").unwrap();
    fs::write(tmp.path().join("doc.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:audio OR type:video").unwrap();
    assert_eq!(results.len(), 2);

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("song.mp3")));
    assert!(paths.iter().any(|p| p.ends_with("clip.mp4")));
}

#[test]
fn test_type_filter_with_not_operator() {
    let tmp = TempDir::new("type_not").unwrap();
    fs::write(tmp.path().join("image.png"), b"x").unwrap();
    fs::write(tmp.path().join("song.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("doc.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("!type:picture").unwrap();
    assert!(results.len() >= 2);

    let has_image = results.iter().any(|&i| {
        cache
            .node_path(i)
            .map(|p| p.ends_with("image.png"))
            .unwrap_or(false)
    });
    assert!(!has_image, "Should not include picture files");
}

#[test]
fn test_type_filter_multiple_extensions_same_category() {
    let tmp = TempDir::new("type_multi_ext").unwrap();
    fs::write(tmp.path().join("photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("graphic.png"), b"x").unwrap();
    fs::write(tmp.path().join("animation.gif"), b"x").unwrap();
    fs::write(tmp.path().join("web.webp"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(results.len(), 4);
}

#[test]
fn test_size_combined_with_type_filter() {
    let tmp = TempDir::new("size_with_type").unwrap();
    fs::write(tmp.path().join("large.png"), vec![0u8; 50_000]).unwrap();
    fs::write(tmp.path().join("small.png"), vec![0u8; 1_000]).unwrap();
    fs::write(tmp.path().join("large.mp3"), vec![0u8; 50_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture size:>10kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("large.png"));
}

#[test]
fn test_type_and_size_complex_query() {
    let tmp = TempDir::new("type_size_complex").unwrap();
    fs::write(tmp.path().join("small_photo.jpg"), vec![0u8; 5_000]).unwrap();
    fs::write(tmp.path().join("large_photo.jpg"), vec![0u8; 50_000]).unwrap();
    fs::write(tmp.path().join("small_video.mp4"), vec![0u8; 5_000]).unwrap();
    fs::write(tmp.path().join("large_video.mp4"), vec![0u8; 50_000]).unwrap();
    fs::write(tmp.path().join("document.pdf"), vec![0u8; 50_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Pictures over 10KB
    let results = cache.search("type:picture size:>10kb").unwrap();
    assert_eq!(results.len(), 1);
    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("large_photo.jpg"));

    // Pictures OR videos, but only over 10KB
    let results2 = cache
        .search("(type:picture OR type:video) size:>10kb")
        .unwrap();
    assert_eq!(results2.len(), 2);

    // Large media files (pictures or videos)
    let results3 = cache
        .search("type:picture OR type:video size:>10kb")
        .unwrap();
    assert_eq!(results3.len(), 2);
}

#[test]
fn test_multiple_type_filters_with_or() {
    let tmp = TempDir::new("multi_type_or").unwrap();
    fs::write(tmp.path().join("song.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("clip.mp4"), b"x").unwrap();
    fs::write(tmp.path().join("photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("doc.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache
        .search("type:audio OR type:video OR type:picture")
        .unwrap();
    assert_eq!(results.len(), 3);

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("song.mp3")));
    assert!(paths.iter().any(|p| p.ends_with("clip.mp4")));
    assert!(paths.iter().any(|p| p.ends_with("photo.jpg")));
}

#[test]
fn test_type_filter_with_parent_filter() {
    let tmp = TempDir::new("type_with_parent").unwrap();
    fs::create_dir(tmp.path().join("images")).unwrap();
    fs::create_dir(tmp.path().join("videos")).unwrap();
    fs::write(tmp.path().join("images/photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("videos/photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("videos/clip.mp4"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let images_dir = tmp.path().join("images");
    let results = cache
        .search(&format!("type:picture parent:{}", images_dir.display()))
        .unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("images/photo.jpg"));
}

#[test]
fn test_type_filter_with_infolder_filter() {
    let tmp = TempDir::new("type_with_infolder").unwrap();
    fs::create_dir(tmp.path().join("media")).unwrap();
    fs::create_dir(tmp.path().join("media/photos")).unwrap();
    fs::write(tmp.path().join("media/song.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("media/photos/pic1.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("media/photos/pic2.png"), b"x").unwrap();
    fs::write(tmp.path().join("doc.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let media_dir = tmp.path().join("media");
    let results = cache
        .search(&format!("type:picture infolder:{}", media_dir.display()))
        .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_type_filter_empty_result() {
    let tmp = TempDir::new("type_empty_result").unwrap();
    fs::write(tmp.path().join("doc.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:audio").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_audio_macro_with_all_extensions() {
    let tmp = TempDir::new("audio_all_ext").unwrap();
    for ext in [
        "mp3", "wav", "flac", "aac", "ogg", "oga", "opus", "wma", "m4a", "alac", "aiff",
    ] {
        fs::write(tmp.path().join(format!("audio.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("audio:").unwrap();
    assert_eq!(results.len(), 11);
}

#[test]
fn test_video_macro_with_all_extensions() {
    let tmp = TempDir::new("video_all_ext").unwrap();
    for ext in [
        "mp4", "m4v", "mov", "avi", "mkv", "wmv", "webm", "flv", "mpg", "mpeg", "3gp", "3g2", "ts",
        "mts", "m2ts",
    ] {
        fs::write(tmp.path().join(format!("video.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("video:").unwrap();
    assert_eq!(results.len(), 15);
}

#[test]
fn test_doc_macro_with_all_extensions() {
    let tmp = TempDir::new("doc_all_ext").unwrap();
    for ext in [
        "txt", "md", "rst", "doc", "docx", "rtf", "odt", "pdf", "pages", "rtfd",
    ] {
        fs::write(tmp.path().join(format!("document.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("doc:").unwrap();
    assert_eq!(results.len(), 10);
}

#[test]
fn test_exe_macro_with_all_extensions() {
    let tmp = TempDir::new("exe_all_ext").unwrap();
    for ext in [
        "exe", "msi", "bat", "cmd", "com", "ps1", "psm1", "app", "apk", "ipa", "jar", "bin", "run",
        "pkg",
    ] {
        fs::write(tmp.path().join(format!("program.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("exe:").unwrap();
    assert_eq!(results.len(), 14);
}

#[test]
fn test_type_categories_overlap() {
    let tmp = TempDir::new("type_overlap").unwrap();
    fs::write(tmp.path().join("document.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // PDF is in both doc and pdf categories
    let doc_results = cache.search("type:doc").unwrap();
    assert_eq!(doc_results.len(), 1);

    let pdf_results = cache.search("type:pdf").unwrap();
    assert_eq!(pdf_results.len(), 1);
}

#[test]
fn test_nested_directory_type_filter() {
    let tmp = TempDir::new("nested_dir_type").unwrap();
    fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
    fs::write(tmp.path().join("a/photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("a/b/photo.png"), b"x").unwrap();
    fs::write(tmp.path().join("a/b/c/photo.gif"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_wildcard_with_type_filter() {
    let tmp = TempDir::new("wildcard_with_type").unwrap();
    fs::write(tmp.path().join("vacation_photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("family_photo.png"), b"x").unwrap();
    fs::write(tmp.path().join("work_doc.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("*photo* type:picture").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_type_filter_files_without_extensions() {
    let tmp = TempDir::new("type_no_ext").unwrap();
    fs::write(tmp.path().join("README"), b"x").unwrap();
    fs::write(tmp.path().join("Makefile"), b"x").unwrap();
    fs::write(tmp.path().join("photo.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(
        results.len(),
        1,
        "Should only match files with picture extensions"
    );
}

#[test]
fn test_type_filter_mixed_case_extensions() {
    let tmp = TempDir::new("type_mixed_case").unwrap();
    fs::write(tmp.path().join("photo1.JPG"), b"x").unwrap();
    fs::write(tmp.path().join("photo2.Jpg"), b"x").unwrap();
    fs::write(tmp.path().join("photo3.jPg"), b"x").unwrap();
    fs::write(tmp.path().join("photo4.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(results.len(), 4, "Should handle all case variations");
}

#[test]
fn test_multiple_type_filters_intersection() {
    let tmp = TempDir::new("multi_type_intersect").unwrap();
    fs::write(tmp.path().join("file.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // A file can't be both audio and video, so intersection should be empty
    let results = cache.search("type:audio type:video").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_type_filter_uppercase_category_names() {
    let tmp = TempDir::new("type_uppercase").unwrap();
    fs::write(tmp.path().join("photo.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:PICTURE").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("type:PiCtUrE").unwrap();
    assert_eq!(results2.len(), 1);
}

#[test]
fn test_type_code_with_dot_prefixed_files() {
    let tmp = TempDir::new("type_code_dot").unwrap();
    fs::write(tmp.path().join(".gitignore"), b"x").unwrap();
    fs::write(tmp.path().join("main.rs"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:code").unwrap();
    assert_eq!(results.len(), 1, "Should match main.rs");
}

#[test]
fn test_type_archive_with_nested_structure() {
    let tmp = TempDir::new("type_archive_nested").unwrap();
    fs::create_dir(tmp.path().join("backups")).unwrap();
    fs::write(tmp.path().join("archive.zip"), b"x").unwrap();
    fs::write(tmp.path().join("backups/backup.tar.gz"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:archive").unwrap();
    // Note: .tar.gz might be recognized as .gz extension
    assert!(!results.is_empty());
}

#[test]
fn test_type_executable_cross_platform() {
    let tmp = TempDir::new("type_exe_cross").unwrap();
    // Windows executables
    fs::write(tmp.path().join("app.exe"), b"x").unwrap();
    fs::write(tmp.path().join("setup.msi"), b"x").unwrap();
    // Unix executables
    fs::write(tmp.path().join("program.bin"), b"x").unwrap();
    fs::write(tmp.path().join("install.run"), b"x").unwrap();
    // macOS
    fs::write(tmp.path().join("Calculator.app"), b"x").unwrap();
    fs::write(tmp.path().join("installer.pkg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:executable").unwrap();
    assert_eq!(results.len(), 6);
}

#[test]
fn test_type_spreadsheet_csv_special_case() {
    let tmp = TempDir::new("type_csv").unwrap();
    fs::write(tmp.path().join("data.csv"), b"x").unwrap();
    fs::write(tmp.path().join("sheet.xlsx"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:spreadsheet").unwrap();
    assert_eq!(results.len(), 2, "CSV should be included in spreadsheets");
}

#[test]
fn test_type_filter_performance_many_files() {
    let tmp = TempDir::new("type_perf").unwrap();
    let extensions = ["jpg", "png", "txt", "pdf", "mp3", "mp4"];
    for i in 0..100 {
        let ext = extensions[i % extensions.len()];
        fs::write(tmp.path().join(format!("file_{i}.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_type_with_extension_that_matches_multiple_categories() {
    let tmp = TempDir::new("type_multi_cat").unwrap();
    // PDF is in multiple categories potentially
    fs::write(tmp.path().join("document.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let doc = cache.search("type:doc").unwrap();
    assert!(!doc.is_empty());

    let pdf = cache.search("type:pdf").unwrap();
    assert!(!pdf.is_empty());
}

#[test]
fn test_type_filter_with_multiple_dots_in_filename() {
    let tmp = TempDir::new("type_multi_dots").unwrap();
    fs::write(tmp.path().join("archive.tar.gz"), b"x").unwrap();
    fs::write(tmp.path().join("backup.tar.bz2"), b"x").unwrap();
    fs::write(tmp.path().join("file.min.js"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Should match based on final extension
    let archives = cache.search("type:archive").unwrap();
    assert!(archives.len() >= 2);

    let code = cache.search("type:code").unwrap();
    assert_eq!(code.len(), 1);
}

#[test]
fn test_type_filter_all_picture_formats() {
    let tmp = TempDir::new("type_all_pictures").unwrap();
    let picture_exts = [
        "jpg", "jpeg", "png", "gif", "bmp", "tif", "tiff", "webp", "ico", "svg", "heic", "heif",
        "raw", "arw", "cr2", "orf", "raf", "psd", "ai",
    ];
    for ext in &picture_exts {
        fs::write(tmp.path().join(format!("image.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(results.len(), picture_exts.len());
}

#[test]
fn test_type_filter_all_video_formats() {
    let tmp = TempDir::new("type_all_videos").unwrap();
    let video_exts = [
        "mp4", "m4v", "mov", "avi", "mkv", "wmv", "webm", "flv", "mpg", "mpeg", "3gp", "3g2", "ts",
        "mts", "m2ts",
    ];
    for ext in &video_exts {
        fs::write(tmp.path().join(format!("video.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:video").unwrap();
    assert_eq!(results.len(), video_exts.len());
}

#[test]
fn test_type_filter_all_audio_formats() {
    let tmp = TempDir::new("type_all_audio").unwrap();
    let audio_exts = [
        "mp3", "wav", "flac", "aac", "ogg", "oga", "opus", "wma", "m4a", "alac", "aiff",
    ];
    for ext in &audio_exts {
        fs::write(tmp.path().join(format!("audio.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:audio").unwrap();
    assert_eq!(results.len(), audio_exts.len());
}

#[test]
fn test_type_filter_all_archive_formats() {
    let tmp = TempDir::new("type_all_archives").unwrap();
    let archive_exts = [
        "zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz", "zst", "cab", "iso", "dmg",
    ];
    for ext in &archive_exts {
        fs::write(tmp.path().join(format!("archive.{ext}")), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:archive").unwrap();
    assert_eq!(results.len(), archive_exts.len());
}

#[test]
fn test_type_code_comprehensive_languages() {
    let tmp = TempDir::new("type_code_langs").unwrap();
    // C family
    fs::write(tmp.path().join("main.c"), b"x").unwrap();
    fs::write(tmp.path().join("impl.cpp"), b"x").unwrap();
    fs::write(tmp.path().join("header.h"), b"x").unwrap();
    // Rust
    fs::write(tmp.path().join("lib.rs"), b"x").unwrap();
    // JavaScript/TypeScript
    fs::write(tmp.path().join("app.js"), b"x").unwrap();
    fs::write(tmp.path().join("component.tsx"), b"x").unwrap();
    // Python
    fs::write(tmp.path().join("script.py"), b"x").unwrap();
    // Go
    fs::write(tmp.path().join("server.go"), b"x").unwrap();
    // Java
    fs::write(tmp.path().join("Main.java"), b"x").unwrap();
    // Web
    fs::write(tmp.path().join("index.html"), b"x").unwrap();
    fs::write(tmp.path().join("style.css"), b"x").unwrap();
    // Config
    fs::write(tmp.path().join("config.json"), b"x").unwrap();
    fs::write(tmp.path().join("data.yaml"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:code").unwrap();
    assert_eq!(results.len(), 13);
}

#[test]
fn test_type_with_uncommon_extensions() {
    let tmp = TempDir::new("type_uncommon").unwrap();
    // Test that uncommon but valid extensions work
    fs::write(tmp.path().join("scan.tiff"), b"x").unwrap();
    fs::write(tmp.path().join("audio.opus"), b"x").unwrap();
    fs::write(tmp.path().join("archive.zst"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let pictures = cache.search("type:picture").unwrap();
    assert_eq!(pictures.len(), 1);

    let audio = cache.search("type:audio").unwrap();
    assert_eq!(audio.len(), 1);

    let archives = cache.search("type:archive").unwrap();
    assert_eq!(archives.len(), 1);
}

#[test]
fn test_type_filter_special_characters_in_names() {
    let tmp = TempDir::new("type_special_chars").unwrap();
    fs::write(tmp.path().join("photo (1).jpg"), b"x").unwrap();
    fs::write(tmp.path().join("song [remix].mp3"), b"x").unwrap();
    fs::write(tmp.path().join("doc & notes.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let pictures = cache.search("type:picture").unwrap();
    assert_eq!(pictures.len(), 1);

    let audio = cache.search("type:audio").unwrap();
    assert_eq!(audio.len(), 1);

    let docs = cache.search("type:doc").unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn test_type_macros_accept_arguments_as_filters() {
    let tmp = TempDir::new("macro_with_args").unwrap();
    fs::write(tmp.path().join("file_match.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("file_skip.mp3"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("audio:match").unwrap();
    assert_eq!(results.len(), 1);
    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("file_match.mp3")));
}

#[test]
fn test_type_filter_with_wildcard_name() {
    let tmp = TempDir::new("type_wildcard_name").unwrap();
    fs::write(tmp.path().join("photo_001.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("photo_002.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("image_003.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("photo* type:picture").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_type_filter_mixed_categories() {
    let tmp = TempDir::new("type_mixed").unwrap();
    fs::write(tmp.path().join("a.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("b.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("c.mp4"), b"x").unwrap();
    fs::write(tmp.path().join("d.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("e.zip"), b"x").unwrap();
    fs::write(tmp.path().join("f.exe"), b"x").unwrap();
    fs::write(tmp.path().join("g.rs"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let picture = cache.search("type:picture").unwrap();
    assert_eq!(picture.len(), 1);

    let audio = cache.search("type:audio").unwrap();
    assert_eq!(audio.len(), 1);

    let video = cache.search("type:video").unwrap();
    assert_eq!(video.len(), 1);

    let doc = cache.search("type:doc").unwrap();
    assert_eq!(doc.len(), 1);

    let archive = cache.search("type:archive").unwrap();
    assert_eq!(archive.len(), 1);

    let exe = cache.search("type:exe").unwrap();
    assert_eq!(exe.len(), 1);

    let code = cache.search("type:code").unwrap();
    assert_eq!(code.len(), 1);
}

#[test]
fn test_type_filter_negation_complex() {
    let tmp = TempDir::new("type_negation").unwrap();
    fs::write(tmp.path().join("image.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("video.mp4"), b"x").unwrap();
    fs::write(tmp.path().join("doc.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("!type:picture !type:video").unwrap();
    let has_image = results.iter().any(|&i| {
        cache
            .node_path(i)
            .map(|p| p.ends_with("image.jpg"))
            .unwrap_or(false)
    });
    let has_video = results.iter().any(|&i| {
        cache
            .node_path(i)
            .map(|p| p.ends_with("video.mp4"))
            .unwrap_or(false)
    });
    assert!(!has_image && !has_video);
}

#[test]
fn test_type_and_size_with_grouping() {
    let tmp = TempDir::new("type_size_grouping").unwrap();
    fs::write(tmp.path().join("large_photo.jpg"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("small_photo.jpg"), vec![0u8; 1_000]).unwrap();
    fs::write(tmp.path().join("large_video.mp4"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("small_video.mp4"), vec![0u8; 1_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache
        .search("(type:picture OR type:video) size:>10kb")
        .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_multiple_ext_and_type_filter() {
    let tmp = TempDir::new("multi_ext_type").unwrap();
    fs::write(tmp.path().join("photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("graphic.png"), b"x").unwrap();
    fs::write(tmp.path().join("document.txt"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // ext: and type: should intersect
    let results = cache.search("ext:jpg;png type:picture").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_type_filter_unicode_filenames() {
    let tmp = TempDir::new("type_unicode").unwrap();
    fs::write(tmp.path().join("照片.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("音乐.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("文档.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let pictures = cache.search("type:picture").unwrap();
    assert_eq!(pictures.len(), 1);

    let audio = cache.search("type:audio").unwrap();
    assert_eq!(audio.len(), 1);

    let docs = cache.search("type:doc").unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn test_type_folder_with_size_error() {
    let tmp = TempDir::new("type_folder_size").unwrap();
    fs::create_dir(tmp.path().join("folder")).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // size: only applies to files, so folders should be excluded
    let results = cache.search("type:folder size:>0").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_type_file_basic() {
    let tmp = TempDir::new("type_file_basic").unwrap();
    fs::write(tmp.path().join("file.txt"), b"x").unwrap();
    fs::create_dir(tmp.path().join("folder")).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:file").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("file.txt"));
}

#[test]
fn test_type_with_hidden_files() {
    let tmp = TempDir::new("type_hidden").unwrap();
    fs::write(tmp.path().join(".hidden.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("visible.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(results.len(), 2, "Should match hidden files too");
}

#[test]
fn test_type_all_alternate_names() {
    let tmp = TempDir::new("type_alt_names").unwrap();
    fs::write(tmp.path().join("image.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Test all alternate names for pictures
    assert_eq!(cache.search("type:picture").unwrap().len(), 1);
    assert_eq!(cache.search("type:pictures").unwrap().len(), 1);
    assert_eq!(cache.search("type:image").unwrap().len(), 1);
    assert_eq!(cache.search("type:images").unwrap().len(), 1);
    assert_eq!(cache.search("type:photo").unwrap().len(), 1);
    assert_eq!(cache.search("type:photos").unwrap().len(), 1);
}

#[test]
fn test_type_with_repeated_filters() {
    let tmp = TempDir::new("type_repeated").unwrap();
    fs::write(tmp.path().join("photo.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Same type filter repeated should still work
    let results = cache.search("type:picture type:picture").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_type_extensions_case_normalization() {
    let tmp = TempDir::new("type_case_norm").unwrap();
    fs::write(tmp.path().join("photo1.JPG"), b"x").unwrap();
    fs::write(tmp.path().join("photo2.JpG"), b"x").unwrap();
    fs::write(tmp.path().join("photo3.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(
        results.len(),
        3,
        "Should match all case variations of JPG extension"
    );
}

#[test]
fn test_type_empty_extension() {
    let tmp = TempDir::new("type_empty_ext").unwrap();
    fs::write(tmp.path().join("file."), b"x").unwrap();
    fs::write(tmp.path().join("normal.jpg"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(
        results.len(),
        1,
        "Should not match file with empty extension"
    );
}

#[test]
fn test_type_multiple_extensions_in_filename() {
    let tmp = TempDir::new("type_multi_ext_name").unwrap();
    fs::write(tmp.path().join("archive.tar.gz"), b"x").unwrap();
    fs::write(tmp.path().join("backup.tar.bz2"), b"x").unwrap();
    fs::write(tmp.path().join("data.json.backup"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Should match based on final extension
    let gz = cache.search("type:archive").unwrap();
    assert!(gz.len() >= 2, "Should match .gz and .bz2");
}

#[test]
fn test_type_and_ext_filter_conflict() {
    let tmp = TempDir::new("type_ext_conflict").unwrap();
    fs::write(tmp.path().join("photo.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("document.pdf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // type:picture AND ext:pdf should give empty result
    let results = cache.search("type:picture ext:pdf").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_type_code_config_files() {
    let tmp = TempDir::new("type_code_config").unwrap();
    fs::write(tmp.path().join("config.json"), b"x").unwrap();
    fs::write(tmp.path().join("settings.yaml"), b"x").unwrap();
    fs::write(tmp.path().join("Cargo.toml"), b"x").unwrap();
    fs::write(tmp.path().join("setup.ini"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:code").unwrap();
    assert_eq!(
        results.len(),
        4,
        "Config files should be included in code type"
    );
}

#[test]
fn test_type_presentation_all_formats() {
    let tmp = TempDir::new("type_pres_all").unwrap();
    fs::write(tmp.path().join("deck.ppt"), b"x").unwrap();
    fs::write(tmp.path().join("slides.pptx"), b"x").unwrap();
    fs::write(tmp.path().join("keynote.key"), b"x").unwrap();
    fs::write(tmp.path().join("present.odp"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:presentation").unwrap();
    assert_eq!(results.len(), 4);
}

#[test]
fn test_type_macros_case_insensitive() {
    let tmp = TempDir::new("macro_case").unwrap();
    fs::write(tmp.path().join("song.mp3"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let lower = cache.search("audio:").unwrap();
    assert_eq!(lower.len(), 1);

    let upper = cache.search("AUDIO:").unwrap();
    assert_eq!(upper.len(), 1);

    let mixed = cache.search("AuDiO:").unwrap();
    assert_eq!(mixed.len(), 1);
}

#[test]
fn test_type_file_and_folder_together() {
    let tmp = TempDir::new("type_both").unwrap();
    fs::write(tmp.path().join("file.txt"), b"x").unwrap();
    fs::create_dir(tmp.path().join("folder")).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // file OR folder should return both
    let results = cache.search("type:file OR type:folder").unwrap();
    assert!(results.len() >= 2, "Should return at least file and folder");
}

#[test]
fn test_type_with_no_extension_edge_case() {
    let tmp = TempDir::new("type_no_ext_edge").unwrap();
    fs::write(tmp.path().join("Makefile"), b"x").unwrap();
    fs::write(tmp.path().join("README"), b"x").unwrap();
    fs::write(tmp.path().join("LICENSE"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // These files have no extensions, so type filters shouldn't match them
    let results = cache.search("type:doc").unwrap();
    assert_eq!(results.len(), 0);

    let results2 = cache.search("type:code").unwrap();
    assert_eq!(results2.len(), 0);
}

#[test]
fn test_type_picture_raw_formats() {
    let tmp = TempDir::new("type_raw").unwrap();
    fs::write(tmp.path().join("sony.arw"), b"x").unwrap();
    fs::write(tmp.path().join("canon.cr2"), b"x").unwrap();
    fs::write(tmp.path().join("olympus.orf"), b"x").unwrap();
    fs::write(tmp.path().join("fuji.raf"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(results.len(), 4, "RAW formats should be recognized");
}

#[test]
fn test_type_video_mobile_formats() {
    let tmp = TempDir::new("type_video_mobile").unwrap();
    fs::write(tmp.path().join("video.3gp"), b"x").unwrap();
    fs::write(tmp.path().join("video.3g2"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:video").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_type_spreadsheet_all_variants() {
    let tmp = TempDir::new("type_sheet_all").unwrap();
    fs::write(tmp.path().join("old.xls"), b"x").unwrap();
    fs::write(tmp.path().join("new.xlsx"), b"x").unwrap();
    fs::write(tmp.path().join("data.csv"), b"x").unwrap();
    fs::write(tmp.path().join("apple.numbers"), b"x").unwrap();
    fs::write(tmp.path().join("open.ods"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:spreadsheet").unwrap();
    assert_eq!(results.len(), 5);
}
