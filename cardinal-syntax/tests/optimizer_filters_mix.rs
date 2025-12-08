mod common;
use cardinal_syntax::*;
use common::*;

#[test]
fn block_06_filters_mix() {
    let s1 = parse_ok("folder:src ext:rs regex:.*\\.rs$");
    let p1 = as_and(&s1);
    assert!(p1.len() >= 3);
    let s2 = parse_ok("ext:rs folder:src dm:today");
    let p2 = as_and(&s2);
    let l2 = p2.len();
    filter_is_kind(&p2[l2 - 1], &FilterKind::DateModified);
    let s3 = parse_ok("dc:pastweek a b c");
    let p3 = as_and(&s3);
    let l3 = p3.len();
    filter_is_kind(&p3[l3 - 1], &FilterKind::DateCreated);
    let s4 = parse_ok("type:picture folder:assets a b");
    let p4 = as_and(&s4);
    assert!(p4.len() >= 3);
    let s5 = parse_ok("doc: a b c dm:today");
    let p5 = as_and(&s5);
    let l5 = p5.len();
    filter_is_kind(&p5[l5 - 1], &FilterKind::DateModified);
    let s6 = parse_ok("video: a b dc:pastweek");
    let p6 = as_and(&s6);
    let l6 = p6.len();
    filter_is_kind(&p6[l6 - 1], &FilterKind::DateCreated);
    let s7 = parse_ok("audio: ext:mp3 a b");
    let p7 = as_and(&s7);
    assert!(p7.len() >= 3);
    let s8 = parse_ok("folder:src !ext:md a");
    let p8 = as_and(&s8);
    assert!(p8.len() >= 2);
    let s9 = parse_ok("folder:src (!ext:md) a");
    let p9 = as_and(&s9);
    assert!(p9.len() >= 2);
    let s10 = parse_ok("(folder:src folder:components) ext:tsx");
    let p10 = as_and(&s10);
    assert!(p10.len() >= 2);
}

#[test]
fn tag_filters_move_to_front() {
    let expr = parse_ok("alpha tag:first beta tag:second ext:txt folder:src");
    let parts = as_and(&expr);
    assert!(parts.len() >= 6);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    word_is(&parts[2], "alpha");
    word_is(&parts[3], "beta");
    filter_is_kind(&parts[parts.len() - 2], &FilterKind::Ext);
    filter_is_kind(&parts[parts.len() - 1], &FilterKind::Folder);
}

#[test]
fn tag_filter_only() {
    let expr = parse_ok("tag:important");
    let term = as_term(&expr);
    match term {
        Term::Filter(f) => assert!(matches!(f.kind, FilterKind::Tag)),
        _ => panic!("expected tag filter"),
    }
}

#[test]
fn tag_filter_with_single_word() {
    let expr = parse_ok("tag:project alpha");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    word_is(&parts[1], "alpha");
}

#[test]
fn multiple_tag_filters_preserve_order() {
    let expr = parse_ok("tag:alpha tag:beta tag:gamma");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    filter_is_kind(&parts[2], &FilterKind::Tag);
}

#[test]
fn tag_filter_at_end_moves_to_front() {
    let expr = parse_ok("alpha beta tag:project");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    word_is(&parts[1], "alpha");
    word_is(&parts[2], "beta");
}

#[test]
fn tag_filter_in_middle_moves_to_front() {
    let expr = parse_ok("alpha tag:project beta");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    word_is(&parts[1], "alpha");
    word_is(&parts[2], "beta");
}

#[test]
fn tag_and_other_filters_ordered_correctly() {
    let expr = parse_ok("ext:txt tag:important dm:today");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Ext);
    filter_is_kind(&parts[2], &FilterKind::DateModified);
}

#[test]
fn tag_filter_with_size_and_type() {
    let expr = parse_ok("size:>1mb tag:archive type:file");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Size);
    filter_is_kind(&parts[2], &FilterKind::Type);
}

#[test]
fn tag_filter_with_parent_and_infolder() {
    let expr = parse_ok("alpha parent:/tmp beta infolder:/home gamma tag:work delta");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 7);

    // Stable sort by priority: tag=0, infolder=1, parent=1
    // parent appears before infolder in source, so stable sort preserves that order
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    filter_is_kind(&parts[2], &FilterKind::InFolder);

    // Remaining words maintain their relative order after the priority filters.
    word_is(&parts[3], "alpha");
    word_is(&parts[4], "beta");
    word_is(&parts[5], "gamma");
    word_is(&parts[6], "delta");
}

#[test]
fn parent_filter_moves_to_front() {
    let expr = parse_ok("alpha parent:/tmp beta");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);

    filter_is_kind(&parts[0], &FilterKind::Parent);
    word_is(&parts[1], "alpha");
    word_is(&parts[2], "beta");
}

#[test]
fn infolder_filter_moves_to_front() {
    let expr = parse_ok("alpha infolder:/tmp beta");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);

    filter_is_kind(&parts[0], &FilterKind::InFolder);
    word_is(&parts[1], "alpha");
    word_is(&parts[2], "beta");
}

#[test]
fn scope_filters_preserve_relative_order() {
    let expr = parse_ok("tag:one alpha parent:/tmp beta infolder:/home gamma");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 6);

    // Stable sort: tag first, then parent and infolder (equal priority, stable order)
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    filter_is_kind(&parts[2], &FilterKind::InFolder);
    word_is(&parts[3], "alpha");
    word_is(&parts[4], "beta");
    word_is(&parts[5], "gamma");
}

#[test]
fn tag_filters_with_words_and_phrases() {
    let expr = parse_ok("alpha tag:proj1 \"beta gamma\" tag:proj2 delta");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 5);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    word_is(&parts[2], "alpha");
    phrase_is(&parts[3], "beta gamma");
    word_is(&parts[4], "delta");
}

#[test]
fn tag_filter_with_regex() {
    let expr = parse_ok("regex:.*\\.txt$ tag:docs");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    regex_is(&parts[1], ".*\\.txt$");
}

#[test]
fn tag_filter_in_not_expression() {
    let expr = parse_ok("alpha !tag:temporary");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    word_is(&parts[0], "alpha");
    let inner = as_not(&parts[1]);
    filter_is_kind(inner, &FilterKind::Tag);
}

#[test]
fn tag_filter_with_all_filter_types() {
    let expr = parse_ok(
        "tag:a file: folder: ext:txt type:doc audio: video: doc: exe: size:>1kb dm:today dc:yesterday parent:/tmp infolder:/home nosubfolders:/data content:needle",
    );
    let parts = as_and(&expr);
    assert!(parts.len() >= 15);

    // First filter should be tag
    filter_is_kind(&parts[0], &FilterKind::Tag);

    // All other filters should be at the end after any words
    for (i, part) in parts.iter().enumerate().skip(1) {
        match as_term(&part) {
            Term::Filter(_) => {}
            _ => panic!("expected filter at position {i}"),
        }
    }
}

#[test]
fn multiple_tags_with_multiple_other_filters() {
    let expr = parse_ok("tag:alpha tag:beta ext:rs size:>10kb tag:gamma dm:today");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 6);

    // All tag filters should be at the front
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    filter_is_kind(&parts[2], &FilterKind::Tag);

    // Other filters at the end
    filter_is_kind(&parts[3], &FilterKind::Ext);
    filter_is_kind(&parts[4], &FilterKind::Size);
    filter_is_kind(&parts[5], &FilterKind::DateModified);
}

#[test]
fn tag_filter_preserves_relative_order_with_other_tags() {
    let expr = parse_ok("word1 tag:first word2 tag:second word3 tag:third");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 6);

    // Tags should be at front in original order
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    filter_is_kind(&parts[2], &FilterKind::Tag);

    // Words after tags
    word_is(&parts[3], "word1");
    word_is(&parts[4], "word2");
    word_is(&parts[5], "word3");
}

#[test]
fn tag_filter_with_nested_groups() {
    let expr = parse_ok("(tag:alpha beta) gamma");
    let parts = as_and(&expr);
    // Optimizer flattens nested AND groups, so (tag:alpha beta) gamma becomes tag:alpha beta gamma
    assert_eq!(parts.len(), 3);

    // Tag filter moved to front
    filter_is_kind(&parts[0], &FilterKind::Tag);
    word_is(&parts[1], "beta");
    word_is(&parts[2], "gamma");
}

#[test]
fn tag_filter_with_or_expression() {
    let expr = parse_ok("tag:alpha | tag:beta");
    let parts = as_or(&expr);
    assert_eq!(parts.len(), 2);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
}

#[test]
fn tag_filter_complex_boolean() {
    let expr = parse_ok("(tag:important | tag:urgent) ext:txt");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);

    // First element is the OR group
    let or_parts = as_or(&parts[0]);
    assert_eq!(or_parts.len(), 2);
    filter_is_kind(&or_parts[0], &FilterKind::Tag);
    filter_is_kind(&or_parts[1], &FilterKind::Tag);

    // Second element is the ext filter
    filter_is_kind(&parts[1], &FilterKind::Ext);
}

#[test]
fn no_filters_stays_unchanged() {
    let expr = parse_ok("alpha beta gamma");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    word_is(&parts[0], "alpha");
    word_is(&parts[1], "beta");
    word_is(&parts[2], "gamma");
}

#[test]
fn only_non_tag_filters() {
    let expr = parse_ok("ext:txt dm:today size:>1kb");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);

    // All should be filters, order preserved
    filter_is_kind(&parts[0], &FilterKind::Ext);
    filter_is_kind(&parts[1], &FilterKind::DateModified);
    filter_is_kind(&parts[2], &FilterKind::Size);
}

#[test]
fn tag_filter_with_empty_query_parts() {
    let expr = parse_ok("  tag:alpha   beta  ");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    word_is(&parts[1], "beta");
}

#[test]
fn tag_filter_with_wildcard() {
    let expr = parse_ok("*.txt tag:docs");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    word_is(&parts[1], "*.txt");
}

// ============ Corner Cases ============

#[test]
fn multiple_priority_filters_with_duplicates() {
    let expr = parse_ok("tag:a parent:/tmp tag:b infolder:/home tag:c parent:/usr");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 6);

    // All tags first (priority 0), then parent/infolder (priority 1) in encounter order
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    filter_is_kind(&parts[2], &FilterKind::Tag);
    filter_is_kind(&parts[3], &FilterKind::Parent);
    filter_is_kind(&parts[4], &FilterKind::InFolder);
    filter_is_kind(&parts[5], &FilterKind::Parent);
}

#[test]
fn all_three_priority_levels_mixed() {
    let expr = parse_ok("word1 ext:txt parent:/a tag:one dm:today infolder:/b word2 tag:two size:>1kb parent:/c");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 10);

    // Priority 0: tags
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    // Priority 1: parent/infolder
    filter_is_kind(&parts[2], &FilterKind::Parent);
    filter_is_kind(&parts[3], &FilterKind::InFolder);
    filter_is_kind(&parts[4], &FilterKind::Parent);
    // Non-filters
    word_is(&parts[5], "word1");
    word_is(&parts[6], "word2");
    // Tail filters
    filter_is_kind(&parts[7], &FilterKind::Ext);
    filter_is_kind(&parts[8], &FilterKind::DateModified);
    filter_is_kind(&parts[9], &FilterKind::Size);
}

#[test]
fn only_tag_filters() {
    let expr = parse_ok("tag:a tag:b tag:c");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    filter_is_kind(&parts[2], &FilterKind::Tag);
}

#[test]
fn only_parent_and_infolder_filters() {
    let expr = parse_ok("parent:/a infolder:/b parent:/c infolder:/d");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 4);

    // Should preserve encounter order since they're equal priority
    filter_is_kind(&parts[0], &FilterKind::Parent);
    filter_is_kind(&parts[1], &FilterKind::InFolder);
    filter_is_kind(&parts[2], &FilterKind::Parent);
    filter_is_kind(&parts[3], &FilterKind::InFolder);
}

#[test]
fn tag_with_phrase_and_regex() {
    let expr = parse_ok("\"hello world\" tag:test regex:^foo.*bar$ tag:second");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 4);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    phrase_is(&parts[2], "hello world");
    regex_is(&parts[3], "^foo.*bar$");
}

#[test]
fn nested_not_with_priority_filters() {
    let expr = parse_ok("word !tag:temp !parent:/tmp");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);

    // word comes first (non-filter), then NOT expressions
    word_is(&parts[0], "word");
    
    // Both NOT expressions should be present (they are non-filters too)
    match &parts[1] {
        Expr::Not(_) => {},
        other => panic!("expected Not expression, got: {other:?}"),
    }
    match &parts[2] {
        Expr::Not(_) => {},
        other => panic!("expected Not expression, got: {other:?}"),
    }
}

#[test]
fn priority_filters_in_or_expression() {
    let expr = parse_ok("tag:a | parent:/tmp | infolder:/home");
    let parts = as_or(&expr);
    assert_eq!(parts.len(), 3);

    // OR doesn't reorder, each operand is independent
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    filter_is_kind(&parts[2], &FilterKind::InFolder);
}

#[test]
fn priority_filters_in_nested_and_groups() {
    let expr = parse_ok("(tag:a word1) (parent:/tmp word2) ext:txt");
    let parts = as_and(&expr);
    // Optimizer flattens nested AND groups
    assert_eq!(parts.len(), 5);

    // Priority filters bubble to front
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    word_is(&parts[2], "word1");
    word_is(&parts[3], "word2");
    filter_is_kind(&parts[4], &FilterKind::Ext);
}

#[test]
fn single_priority_filter_with_many_tail_filters() {
    let expr = parse_ok("ext:rs size:>1kb dm:today dc:yesterday tag:one type:file");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 6);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Ext);
    filter_is_kind(&parts[2], &FilterKind::Size);
    filter_is_kind(&parts[3], &FilterKind::DateModified);
    filter_is_kind(&parts[4], &FilterKind::DateCreated);
    filter_is_kind(&parts[5], &FilterKind::Type);
}

#[test]
fn empty_tag_argument() {
    let expr = parse_ok("tag: word");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    word_is(&parts[1], "word");
}

#[test]
fn priority_filters_with_comparison_and_range() {
    let expr = parse_ok("parent:/tmp size:>1gb..10gb infolder:/home dm:2024/1/1-2024/12/31 tag:work");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 5);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    filter_is_kind(&parts[2], &FilterKind::InFolder);
    filter_is_kind(&parts[3], &FilterKind::Size);
    filter_is_kind(&parts[4], &FilterKind::DateModified);
}

#[test]
fn interleaved_priority_and_tail_filters() {
    let expr = parse_ok("ext:txt tag:a size:>1kb parent:/tmp dm:today infolder:/home type:file tag:b");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 8);

    // Tags first
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    // Parent/infolder next
    filter_is_kind(&parts[2], &FilterKind::Parent);
    filter_is_kind(&parts[3], &FilterKind::InFolder);
    // Tail filters last, in original order
    filter_is_kind(&parts[4], &FilterKind::Ext);
    filter_is_kind(&parts[5], &FilterKind::Size);
    filter_is_kind(&parts[6], &FilterKind::DateModified);
    filter_is_kind(&parts[7], &FilterKind::Type);
}

#[test]
fn all_filter_types_comprehensive() {
    let expr = parse_ok(
        "word1 tag:a file: folder: ext:txt type:doc audio: video: doc: exe: \
         size:>1kb dm:today dc:yesterday da:lastweek dr:thismonth parent:/tmp \
         infolder:/home nosubfolders:/data child:*.mp3 attrib:H attribdupe: \
         dmdupe: dupe: namepartdupe: sizedupe: artist:Beatles album:Abbey title:Come \
         genre:Rock year:1969 track:01 comment:Remastered width:>1920 height:>1080 \
         dimensions:1920x1080 orientation:landscape bitdepth:24 case:Test content:error \
         nowholefilename:report tag:b parent:/usr tag:c word2",
    );
    let parts = as_and(&expr);

    // First should be all tags (priority 0)
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    filter_is_kind(&parts[2], &FilterKind::Tag);

    // Then parent/infolder (priority 1)
    filter_is_kind(&parts[3], &FilterKind::Parent);
    filter_is_kind(&parts[4], &FilterKind::InFolder);
    filter_is_kind(&parts[5], &FilterKind::Parent);

    // Then words
    let word_start_idx = 6;
    word_is(&parts[word_start_idx], "word1");
    word_is(&parts[word_start_idx + 1], "word2");

    // Rest are tail filters in order
    let tail_start = word_start_idx + 2;
    assert!(parts.len() > tail_start);
    for part in &parts[tail_start..] {
        // All remaining should be filters
        match as_term(part) {
            Term::Filter(_) => {}
            other => panic!("expected filter, got: {other:?}"),
        }
    }
}

#[test]
fn quoted_priority_filter_arguments() {
    let expr = parse_ok("parent:\"/Users/My Documents\" tag:\"Work Projects\" infolder:\"/home/user/files\"");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    filter_is_kind(&parts[2], &FilterKind::InFolder);
}

#[test]
fn priority_filters_with_wildcards_in_arguments() {
    let expr = parse_ok("tag:proj* parent:/tmp/* infolder:/home/user/*");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    filter_is_kind(&parts[2], &FilterKind::InFolder);
}

#[test]
fn mixed_or_and_and_with_priority_filters() {
    let expr = parse_ok("(tag:urgent | tag:important) word parent:/tmp ext:txt");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 4);

    // OR group is not a filter, comes after priority filters
    filter_is_kind(&parts[0], &FilterKind::Parent);
    let or_parts = as_or(&parts[1]);
    assert_eq!(or_parts.len(), 2);
    filter_is_kind(&or_parts[0], &FilterKind::Tag);
    filter_is_kind(&or_parts[1], &FilterKind::Tag);
    word_is(&parts[2], "word");
    filter_is_kind(&parts[3], &FilterKind::Ext);
}

#[test]
fn deeply_nested_groups_with_priority_filters() {
    let expr = parse_ok("((tag:a word1) word2) parent:/tmp");
    let parts = as_and(&expr);
    // Flattened: tag:a word1 word2 parent:/tmp
    assert_eq!(parts.len(), 4);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    word_is(&parts[2], "word1");
    word_is(&parts[3], "word2");
}

#[test]
fn priority_filter_at_every_position() {
    // Test tag at beginning, middle, end
    let expr1 = parse_ok("tag:start word1 word2");
    let p1 = as_and(&expr1);
    assert_eq!(p1.len(), 3);
    filter_is_kind(&p1[0], &FilterKind::Tag);

    let expr2 = parse_ok("word1 tag:middle word2");
    let p2 = as_and(&expr2);
    assert_eq!(p2.len(), 3);
    filter_is_kind(&p2[0], &FilterKind::Tag);

    let expr3 = parse_ok("word1 word2 tag:end");
    let p3 = as_and(&expr3);
    assert_eq!(p3.len(), 3);
    filter_is_kind(&p3[0], &FilterKind::Tag);
}

#[test]
fn priority_filters_only_no_other_terms() {
    let expr = parse_ok("tag:a tag:b parent:/tmp infolder:/home parent:/usr");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 5);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    filter_is_kind(&parts[2], &FilterKind::Parent);
    filter_is_kind(&parts[3], &FilterKind::InFolder);
    filter_is_kind(&parts[4], &FilterKind::Parent);
}

#[test]
fn single_word_with_all_filter_types() {
    let expr = parse_ok("word tag:a parent:/tmp infolder:/home ext:txt");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 5);

    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Parent);
    filter_is_kind(&parts[2], &FilterKind::InFolder);
    word_is(&parts[3], "word");
    filter_is_kind(&parts[4], &FilterKind::Ext);
}

#[test]
fn alternating_priority_and_non_priority() {
    let expr = parse_ok("tag:a ext:rs parent:/tmp size:>1kb infolder:/home dm:today tag:b type:file");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 8);

    // Priority filters at front
    filter_is_kind(&parts[0], &FilterKind::Tag);
    filter_is_kind(&parts[1], &FilterKind::Tag);
    filter_is_kind(&parts[2], &FilterKind::Parent);
    filter_is_kind(&parts[3], &FilterKind::InFolder);

    // Tail filters preserve order
    filter_is_kind(&parts[4], &FilterKind::Ext);
    filter_is_kind(&parts[5], &FilterKind::Size);
    filter_is_kind(&parts[6], &FilterKind::DateModified);
    filter_is_kind(&parts[7], &FilterKind::Type);
}
