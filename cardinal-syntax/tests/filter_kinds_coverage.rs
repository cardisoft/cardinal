use cardinal_syntax::*;

fn parse_filter(name: &str, arg: Option<&str>) -> Filter {
    let q = if let Some(a) = arg {
        format!("{name}:{a}")
    } else {
        format!("{name}:")
    };
    match parse_query(&q).unwrap().expr {
        Expr::Term(Term::Filter(f)) => f,
        other => panic!("expected filter, got {other:?}"),
    }
}

#[test]
fn maps_known_filter_names() {
    let cases: &[(&str, FilterKind)] = &[
        ("file", FilterKind::File),
        ("folder", FilterKind::Folder),
        ("ext", FilterKind::Ext),
        ("type", FilterKind::Type),
        ("audio", FilterKind::Audio),
        ("video", FilterKind::Video),
        ("doc", FilterKind::Doc),
        ("exe", FilterKind::Exe),
        ("size", FilterKind::Size),
        ("dm", FilterKind::DateModified),
        ("datemodified", FilterKind::DateModified),
        ("dc", FilterKind::DateCreated),
        ("datecreated", FilterKind::DateCreated),
        ("da", FilterKind::DateAccessed),
        ("dateaccessed", FilterKind::DateAccessed),
        ("dr", FilterKind::DateRun),
        ("daterun", FilterKind::DateRun),
        ("parent", FilterKind::Parent),
        ("infolder", FilterKind::InFolder),
        ("nosubfolders", FilterKind::NoSubfolders),
        ("child", FilterKind::Child),
        ("attrib", FilterKind::Attribute),
        ("attribdupe", FilterKind::AttributeDuplicate),
        ("dmdupe", FilterKind::DateModifiedDuplicate),
        ("dupe", FilterKind::Duplicate),
        ("namepartdupe", FilterKind::NamePartDuplicate),
        ("sizedupe", FilterKind::SizeDuplicate),
        ("artist", FilterKind::Artist),
        ("album", FilterKind::Album),
        ("title", FilterKind::Title),
        ("genre", FilterKind::Genre),
        ("year", FilterKind::Year),
        ("track", FilterKind::Track),
        ("comment", FilterKind::Comment),
        ("width", FilterKind::Width),
        ("height", FilterKind::Height),
        ("dimensions", FilterKind::Dimensions),
        ("orientation", FilterKind::Orientation),
        ("bitdepth", FilterKind::BitDepth),
        ("case", FilterKind::CaseSensitive),
        ("content", FilterKind::Content),
        ("nowholefilename", FilterKind::NoWholeFilename),
    ];

    for (name, expected) in cases {
        let f = parse_filter(name, None);
        assert_eq!(&f.kind, expected, "name={name}");
        assert!(f.argument.is_none());
    }
}

#[test]
fn preserves_custom_names() {
    let f = parse_filter("proj", None);
    match f.kind {
        FilterKind::Custom(n) => assert_eq!(n, "proj"),
        other => panic!("{other:?}"),
    }

    let f = parse_filter("D", None);
    match f.kind {
        FilterKind::Custom(n) => assert_eq!(n, "D"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn argument_shapes_overview() {
    // list
    let f = parse_filter("ext", Some("jpg;png;gif"));
    assert!(matches!(f.argument.unwrap().kind, ArgumentKind::List(_)));

    // range dotted
    let f = parse_filter("size", Some("1..10"));
    assert!(matches!(f.argument.unwrap().kind, ArgumentKind::Range(_)));

    // comparison
    let f = parse_filter("size", Some(">=100"));
    assert!(matches!(
        f.argument.unwrap().kind,
        ArgumentKind::Comparison(_)
    ));

    // phrase
    let f = parse_filter("parent", Some("\"/Users/demo\""));
    assert!(matches!(f.argument.unwrap().kind, ArgumentKind::Phrase));

    // bare
    let f = parse_filter("content", Some("error"));
    assert!(matches!(f.argument.unwrap().kind, ArgumentKind::Bare));
}
