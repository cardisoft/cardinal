use crate::{
    SearchCache, SearchOptions, SegmentKind, SegmentMatcher, SegmentMatcherConcrete, SlabIndex,
    SlabNodeMetadataCompact, build_segment_matchers, cache::NAME_POOL,
};
use anyhow::{Result, anyhow, bail};
use cardinal_syntax::{
    ArgumentKind, ComparisonOp, Expr, Filter, FilterArgument, FilterKind, RangeSeparator, Term,
};
use fswalk::NodeFileType;
use hashbrown::HashSet;
use jiff::{Timestamp, civil::Date, tz::TimeZone};
use memchr::arch::all::rabinkarp;
use query_segmentation::query_segmentation;
use rayon::iter::{ParallelBridge, ParallelIterator};
use regex::RegexBuilder;
use search_cancel::CancellationToken;
use std::{collections::BTreeSet, fs::File, io::Read, path::Path};

pub(crate) const CONTENT_BUFFER_BYTES: usize = 64 * 1024;

impl SearchCache {
    pub(crate) fn evaluate_expr(
        &mut self,
        expr: &Expr,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        match expr {
            Expr::Empty => Ok(self.search_empty(token)),
            Expr::Term(term) => self.evaluate_term(term, options, token),
            Expr::Not(inner) => self.evaluate_not(inner, None, options, token),
            Expr::And(parts) => self.evaluate_and(parts, options, token),
            Expr::Or(parts) => self.evaluate_or(parts, options, token),
        }
    }

    fn evaluate_and(
        &mut self,
        parts: &[Expr],
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let mut current: Option<Vec<SlabIndex>> = None;
        for part in parts {
            match part {
                Expr::Not(inner) => {
                    let Some(x) = self.evaluate_not(inner, current, options, token)? else {
                        return Ok(None);
                    };
                    current = Some(x);
                }
                Expr::Term(Term::Filter(filter)) => {
                    let base = current.take();
                    let Some(nodes) = self.evaluate_filter(filter, base, options, token)? else {
                        return Ok(None);
                    };
                    current = Some(nodes);
                }
                _ => {
                    let Some(nodes) = self.evaluate_expr(part, options, token)? else {
                        return Ok(None);
                    };
                    current = Some(match current {
                        Some(mut existing) => {
                            if intersect_in_place(&mut existing, &nodes, token).is_none() {
                                return Ok(None);
                            }
                            existing
                        }
                        None => nodes,
                    });
                }
            }
        }
        Ok(Some(current.expect("at least one part in AND expression")))
    }

    fn evaluate_or(
        &mut self,
        parts: &[Expr],
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let mut result: Vec<SlabIndex> = Vec::new();
        for part in parts {
            let candidate = self.evaluate_expr(part, options, token)?;
            let Some(nodes) = candidate else {
                return Ok(None);
            };
            if union_in_place(&mut result, &nodes, token).is_none() {
                return Ok(None);
            }
        }
        Ok(Some(result))
    }

    fn evaluate_not(
        &mut self,
        inner: &Expr,
        base: Option<Vec<SlabIndex>>,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let mut universe = if let Some(current) = base {
            current
        } else {
            match self.search_empty(token) {
                Some(nodes) => nodes,
                None => return Ok(None),
            }
        };
        if let Some(negated) = self.evaluate_expr(inner, options, token)? {
            if difference_in_place(&mut universe, &negated, token).is_none() {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
        Ok(Some(universe))
    }

    fn evaluate_term(
        &mut self,
        term: &Term,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        match term {
            Term::Word(text) => self.evaluate_word(text, options, token),
            Term::Phrase(text) => self.evaluate_phrase(text, options, token),
            Term::Regex(pattern) => self.evaluate_regex(pattern, options, token),
            Term::Filter(filter) => self.evaluate_filter(filter, None, options, token),
        }
    }

    fn evaluate_word(
        &self,
        text: &str,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        self.evaluate_phrase(text, options, token)
    }

    fn evaluate_phrase(
        &self,
        text: &str,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let segments = query_segmentation(text);
        if segments.is_empty() {
            bail!("Unprocessable term: {text:?}");
        }
        let matchers = build_segment_matchers(&segments, options)
            .map_err(|err| anyhow!("Invalid regex pattern: {err}"))?;
        Ok(self.execute_matchers(&matchers, token))
    }

    fn execute_matchers(
        &self,
        matchers: &[SegmentMatcher],
        token: CancellationToken,
    ) -> Option<Vec<SlabIndex>> {
        let mut node_set: Option<Vec<SlabIndex>> = None;
        let mut pending_globstar = false;
        let mut saw_matcher = false;
        for matcher in matchers {
            match matcher {
                SegmentMatcher::GlobStar => {
                    pending_globstar = true;
                }
                SegmentMatcher::Concrete(concrete) => {
                    saw_matcher = true;
                    let new_node_set = if let Some(nodes) = &node_set {
                        if pending_globstar {
                            self.match_descendant_segments(nodes, concrete, token)
                        } else {
                            self.match_direct_child_segments(nodes, concrete, token)
                        }
                    } else {
                        self.match_initial_segment(concrete, token)
                    }?;
                    node_set = Some(new_node_set);
                    pending_globstar = false;
                }
            }
        }

        if pending_globstar {
            if let Some(nodes) = node_set.take() {
                Some(self.expand_trailing_globstar(nodes, token)?)
            } else if saw_matcher {
                node_set
            } else {
                self.search_empty(token)
            }
        } else if saw_matcher {
            node_set
        } else {
            self.search_empty(token)
        }
    }

    fn match_initial_segment(
        &self,
        matcher: &SegmentMatcherConcrete,
        token: CancellationToken,
    ) -> Option<Vec<SlabIndex>> {
        let names: BTreeSet<_> = match matcher {
            SegmentMatcherConcrete::Plain { kind, needle } => match kind {
                SegmentKind::Substr => NAME_POOL.search_substr(needle, token),
                SegmentKind::Prefix => NAME_POOL.search_prefix(needle, token),
                SegmentKind::Suffix => NAME_POOL.search_suffix(needle, token),
                SegmentKind::Exact => NAME_POOL.search_exact(needle, token),
            },
            SegmentMatcherConcrete::Regex { regex } => NAME_POOL.search_regex(regex, token),
        }?;
        let mut nodes = Vec::with_capacity(names.len());
        for (i, name) in names.iter().enumerate() {
            token.is_cancelled_sparse(i)?;
            if let Some(indices) = self.name_index.get(name) {
                nodes.extend(indices.iter().copied());
            }
        }
        Some(nodes)
    }

    fn match_direct_child_segments(
        &self,
        parents: &[SlabIndex],
        matcher: &SegmentMatcherConcrete,
        token: CancellationToken,
    ) -> Option<Vec<SlabIndex>> {
        let mut new_node_set = Vec::new();
        for (i, &node) in parents.iter().enumerate() {
            token.is_cancelled_sparse(i)?;
            let mut child_matches = self.file_nodes[node]
                .children
                .iter()
                .filter_map(|&child| {
                    let name = self.file_nodes[child].name_and_parent.as_str();
                    if matcher.matches(name) {
                        Some((name, child))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            child_matches.sort_unstable_by_key(|(name, _)| *name);
            new_node_set.extend(child_matches.into_iter().map(|(_, index)| index));
        }
        Some(new_node_set)
    }

    fn match_descendant_segments(
        &self,
        parents: &[SlabIndex],
        matcher: &SegmentMatcherConcrete,
        token: CancellationToken,
    ) -> Option<Vec<SlabIndex>> {
        let mut matches = Vec::new();
        let mut visited = 0usize;
        for &node in parents {
            token.is_cancelled_sparse(visited)?;
            let descendants = self.all_subnodes(node, token)?;
            for descendant in descendants {
                token.is_cancelled_sparse(visited)?;
                visited += 1;
                let name = self.file_nodes[descendant].name_and_parent.as_str();
                if matcher.matches(name) {
                    matches.push((name, descendant));
                }
            }
        }
        matches.sort_unstable_by_key(|(name, _)| *name);
        Some(matches.into_iter().map(|(_, index)| index).collect())
    }

    fn expand_trailing_globstar(
        &self,
        mut nodes: Vec<SlabIndex>,
        token: CancellationToken,
    ) -> Option<Vec<SlabIndex>> {
        if nodes.is_empty() {
            return Some(nodes);
        }
        let base = nodes.clone();
        let mut extra = Vec::new();
        let mut visited = 0usize;
        for &node in &base {
            token.is_cancelled_sparse(visited)?;
            let descendants = self.all_subnodes(node, token)?;
            for descendant in descendants {
                token.is_cancelled_sparse(visited)?;
                visited += 1;
                extra.push(descendant);
            }
        }
        nodes.extend(extra);
        Some(nodes)
    }

    fn evaluate_regex(
        &self,
        pattern: &str,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let mut builder = RegexBuilder::new(pattern);
        builder.case_insensitive(options.case_insensitive);
        let regex = builder
            .build()
            .map_err(|err| anyhow!("Invalid regex pattern: {err}"))?;
        let matcher = SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex });
        Ok(self.execute_matchers(std::slice::from_ref(&matcher), token))
    }

    fn evaluate_filter(
        &mut self,
        filter: &Filter,
        base: Option<Vec<SlabIndex>>,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        match filter.kind {
            FilterKind::File => self.evaluate_type_filter(
                NodeFileType::File,
                base,
                filter.argument.as_ref(),
                options,
                token,
            ),
            FilterKind::Folder => self.evaluate_type_filter(
                NodeFileType::Dir,
                base,
                filter.argument.as_ref(),
                options,
                token,
            ),
            FilterKind::Ext => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("ext: requires at least one extension"))?;
                self.evaluate_extension_filter(argument, base, token)
            }
            FilterKind::Parent => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("parent: requires a folder path"))?;
                self.evaluate_parent_filter(argument, base, token)
            }
            FilterKind::InFolder => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("infolder: requires a folder path"))?;
                self.evaluate_infolder_filter(argument, base, token)
            }
            FilterKind::NoSubfolders => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("nosubfolders: requires a folder path"))?;
                self.evaluate_nosubfolders_filter(argument, base, token)
            }
            FilterKind::Type => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("type: requires a category"))?;
                self.evaluate_named_type_filter(&argument.raw, base, options, token)
            }
            FilterKind::Audio => {
                self.evaluate_type_macro("audio", base, filter.argument.as_ref(), options, token)
            }
            FilterKind::Video => {
                self.evaluate_type_macro("video", base, filter.argument.as_ref(), options, token)
            }
            FilterKind::Doc => {
                self.evaluate_type_macro("doc", base, filter.argument.as_ref(), options, token)
            }
            FilterKind::Exe => {
                self.evaluate_type_macro("exe", base, filter.argument.as_ref(), options, token)
            }
            FilterKind::Size => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("size: requires a value"))?;
                self.evaluate_size_filter(argument, base, token)
            }
            FilterKind::DateModified => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("dm: requires a date or range"))?;
                self.evaluate_date_filter(DateField::Modified, argument, base, token)
            }
            FilterKind::DateCreated => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("dc: requires a date or range"))?;
                self.evaluate_date_filter(DateField::Created, argument, base, token)
            }
            FilterKind::Content => {
                let argument = filter
                    .argument
                    .as_ref()
                    .ok_or_else(|| anyhow!("content: requires a value"))?;
                self.evaluate_content_filter(argument, base, options, token)
            }
            _ => bail!("Filter {:?} is not supported yet", filter.kind),
        }
    }

    fn evaluate_type_filter(
        &self,
        file_type: NodeFileType,
        base: Option<Vec<SlabIndex>>,
        argument: Option<&FilterArgument>,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let (mut nodes, argument_applied) = match (base, argument) {
            (Some(nodes), _) => (nodes, false),
            (None, Some(arg)) => match self.evaluate_phrase(&arg.raw, options, token)? {
                Some(nodes) => (nodes, true),
                None => return Ok(None),
            },
            (None, None) => match self.search_empty(token) {
                Some(nodes) => (nodes, false),
                None => return Ok(None),
            },
        };

        if !argument_applied {
            if let Some(arg) = argument {
                let Some(matches) = self.evaluate_phrase(&arg.raw, options, token)? else {
                    return Ok(None);
                };
                if intersect_in_place(&mut nodes, &matches, token).is_none() {
                    return Ok(None);
                }
            }
        }

        Ok(filter_nodes(nodes, token, |index| {
            self.file_nodes[index].metadata.file_type_hint() == file_type
        }))
    }

    fn evaluate_extension_filter(
        &self,
        argument: &FilterArgument,
        base: Option<Vec<SlabIndex>>,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let extensions = normalize_extensions(argument);
        if extensions.is_empty() {
            bail!("ext: requires non-empty extensions");
        }
        let Some(nodes) = self.nodes_from_base(base, token) else {
            return Ok(None);
        };
        Ok(filter_nodes(nodes, token, |index| {
            let node = &self.file_nodes[index];
            if node.metadata.file_type_hint() != NodeFileType::File {
                return false;
            }
            extension_of(node.name_and_parent.as_str())
                .map(|ext| extensions.contains(ext.as_str()))
                .unwrap_or(false)
        }))
    }

    fn evaluate_parent_filter(
        &self,
        argument: &FilterArgument,
        base: Option<Vec<SlabIndex>>,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let Some(target) = self.node_index_for_raw_path(Path::new(&argument.raw)) else {
            bail!(
                "Parent filter {:?} is not found in file system",
                argument.raw
            );
        };
        let children = self.file_nodes[target].children.to_vec();
        if let Some(mut nodes) = base {
            if intersect_in_place(&mut nodes, &children, token).is_none() {
                return Ok(None);
            }
            Ok(Some(nodes))
        } else {
            Ok(Some(children))
        }
    }

    fn evaluate_infolder_filter(
        &self,
        argument: &FilterArgument,
        base: Option<Vec<SlabIndex>>,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let Some(target) = self.node_index_for_raw_path(Path::new(&argument.raw)) else {
            bail!(
                "Parent filter {:?} is not found in file system",
                argument.raw
            );
        };
        let Some(children) = self.all_subnodes(target, token) else {
            return Ok(None);
        };
        if let Some(mut nodes) = base {
            if intersect_in_place(&mut nodes, &children, token).is_none() {
                return Ok(None);
            }
            Ok(Some(nodes))
        } else {
            Ok(Some(children))
        }
    }

    fn evaluate_nosubfolders_filter(
        &self,
        argument: &FilterArgument,
        base: Option<Vec<SlabIndex>>,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let Some(target) = self.node_index_for_raw_path(Path::new(&argument.raw)) else {
            bail!(
                "nosubfolders filter {:?} is not found in file system",
                argument.raw
            );
        };
        if self.file_nodes[target].metadata.file_type_hint() != NodeFileType::Dir {
            bail!("nosubfolders path {:?} is not a folder", argument.raw);
        }

        let nodes = if let Some(nodes) = base
            && nodes.len() <= self.file_nodes[target].children.len()
        {
            nodes
        } else {
            self.file_nodes[target].children.to_vec()
        };

        Ok(filter_nodes(nodes, token, |index| {
            self.keep_node_for_nosubfolders(index, target)
        }))
    }

    fn keep_node_for_nosubfolders(&self, index: SlabIndex, root: SlabIndex) -> bool {
        index == root || {
            let node = &self.file_nodes[index];
            node.name_and_parent.parent() == Some(root)
                && node.metadata.file_type_hint() != NodeFileType::Dir
        }
    }

    fn evaluate_named_type_filter(
        &self,
        raw: &str,
        base: Option<Vec<SlabIndex>>,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let name = raw.trim();
        if name.is_empty() {
            bail!("type: requires a category");
        }
        let normalized = name.to_ascii_lowercase();
        let Some(target) = lookup_type_group(&normalized) else {
            bail!("Unknown type category: {name}");
        };
        self.apply_type_group(target, base, options, token)
    }

    fn evaluate_type_macro(
        &self,
        name: &'static str,
        base: Option<Vec<SlabIndex>>,
        argument: Option<&FilterArgument>,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let group_nodes = self.apply_type_group(
            lookup_type_group(name).expect("built-in macro should map to a known type group"),
            base,
            options,
            token,
        )?;
        let Some(mut nodes) = group_nodes else {
            return Ok(None);
        };
        let Some(argument) = argument else {
            return Ok(Some(nodes));
        };
        let Some(matches) = self.evaluate_phrase(&argument.raw, options, token)? else {
            return Ok(None);
        };
        if intersect_in_place(&mut nodes, &matches, token).is_none() {
            return Ok(None);
        }
        Ok(Some(nodes))
    }

    fn apply_type_group(
        &self,
        target: TypeFilterTarget,
        base: Option<Vec<SlabIndex>>,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        match target {
            TypeFilterTarget::NodeType(file_type) => {
                self.evaluate_type_filter(file_type, base, None, options, token)
            }
            TypeFilterTarget::Extensions(list) => self.filter_static_extensions(list, base, token),
        }
    }

    fn filter_static_extensions(
        &self,
        extensions: &'static [&'static str],
        base: Option<Vec<SlabIndex>>,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        if extensions.is_empty() {
            return Ok(Some(Vec::new()));
        }
        let Some(nodes) = self.nodes_from_base(base, token) else {
            return Ok(None);
        };
        Ok(filter_nodes(nodes, token, |index| {
            let node = &self.file_nodes[index];
            if node.metadata.file_type_hint() != NodeFileType::File {
                return false;
            }
            if let Some(ext) = extension_of(node.name_and_parent.as_str()) {
                extensions.iter().any(|needle| *needle == ext)
            } else {
                false
            }
        }))
    }

    fn evaluate_size_filter(
        &mut self,
        argument: &FilterArgument,
        base: Option<Vec<SlabIndex>>,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let predicate = SizePredicate::parse(argument)?;
        let Some(nodes) = self.nodes_from_base(base, token) else {
            return Ok(None);
        };
        Ok(filter_nodes(nodes, token, |index| {
            let node = &self.file_nodes[index];
            if node.metadata.file_type_hint() != NodeFileType::File {
                return false;
            }
            let Some(size) = self.node_size_bytes(index) else {
                return false;
            };
            predicate.matches(size)
        }))
    }

    fn evaluate_date_filter(
        &mut self,
        field: DateField,
        argument: &FilterArgument,
        base: Option<Vec<SlabIndex>>,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let context = DateContext::capture();
        let predicate = DatePredicate::parse(argument, &context)?;
        let Some(nodes) = self.nodes_from_base(base, token) else {
            return Ok(None);
        };
        Ok(filter_nodes(nodes, token, |index| {
            let Some(timestamp) = self.node_timestamp(index, field) else {
                return false;
            };
            predicate.matches(timestamp)
        }))
    }

    fn evaluate_content_filter(
        &mut self,
        argument: &FilterArgument,
        base: Option<Vec<SlabIndex>>,
        options: SearchOptions,
        token: CancellationToken,
    ) -> Result<Option<Vec<SlabIndex>>> {
        let ghost;
        let needle = if options.case_insensitive {
            ghost = argument.raw.to_ascii_lowercase().into_bytes();
            &ghost
        } else {
            argument.raw.as_bytes()
        };
        if needle.is_empty() {
            bail!("content: requires a value");
        }

        let Some(nodes) = self.nodes_from_base(base, token) else {
            return Ok(None);
        };

        let matched_indices = nodes
            .into_iter()
            .filter(|index| self.file_nodes[*index].metadata.file_type_hint() == NodeFileType::File)
            .filter_map(|index| self.node_path(index).map(|path| (index, path)))
            .par_bridge()
            .filter_map(|(index, path)| {
                self.node_content_matches(&path, needle, options.case_insensitive, token)?
                    .then_some(index)
            })
            .collect();

        Ok(token.is_cancelled().map(|()| matched_indices))
    }

    /// user need to ensure that needle is lowercased when case_insensitive is set
    fn node_content_matches(
        &self,
        path: &Path,
        needle: &[u8],
        case_insensitive: bool,
        token: CancellationToken,
    ) -> Option<bool> {
        token.is_cancelled()?;

        let Ok(mut file) = File::open(path) else {
            return Some(false);
        };

        if needle.len() == 1 {
            let needle = needle[0];
            let mut buffer = vec![0u8; CONTENT_BUFFER_BYTES];
            if case_insensitive {
                let lowercase_target = needle.to_ascii_lowercase();
                let uppercase_target = needle.to_ascii_uppercase();
                loop {
                    token.is_cancelled()?;
                    let read = match file.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => count,
                        Err(_) => return Some(false),
                    };
                    if buffer[..read]
                        .iter()
                        .any(|&c| c == lowercase_target || c == uppercase_target)
                    {
                        return Some(true);
                    }
                }
            } else {
                loop {
                    token.is_cancelled()?;
                    let read = match file.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => count,
                        Err(_) => return Some(false),
                    };
                    if buffer[..read].contains(&needle) {
                        return Some(true);
                    }
                }
            };

            return Some(false);
        }

        // ensure needle is lowercased if case_insensitive is set
        if case_insensitive {
            debug_assert_eq!(needle, needle.to_ascii_lowercase());
        }
        let overlap = needle.len().saturating_sub(1);
        let finder = rabinkarp::Finder::new(needle);
        let mut buffer = vec![0u8; CONTENT_BUFFER_BYTES + overlap];
        let mut carry_len = 0usize;

        loop {
            token.is_cancelled()?;

            let Ok(read) = file.read(&mut buffer[carry_len..]) else {
                return Some(false);
            };
            if read == 0 {
                break;
            }

            let chunk_len = carry_len + read;
            let chunk = &mut buffer[..chunk_len];

            if case_insensitive {
                chunk[carry_len..].make_ascii_lowercase();
            }

            if finder.find(chunk, needle).is_some() {
                return Some(true);
            }

            let keep = overlap.min(chunk.len());
            if keep > 0 {
                let start = chunk.len().saturating_sub(keep);
                chunk.copy_within(start.., 0);
            }
            carry_len = keep;
        }

        Some(false)
    }

    fn nodes_from_base(
        &self,
        base: Option<Vec<SlabIndex>>,
        token: CancellationToken,
    ) -> Option<Vec<SlabIndex>> {
        match base {
            Some(nodes) => Some(nodes),
            None => self.search_empty(token),
        }
    }

    fn node_size_bytes(&mut self, index: SlabIndex) -> Option<u64> {
        self.ensure_metadata(index).as_ref().map(|x| x.size())
    }

    fn node_timestamp(&mut self, index: SlabIndex, field: DateField) -> Option<i64> {
        let metadata = self.ensure_metadata(index);
        let meta = metadata.as_ref()?;
        match field {
            DateField::Modified => meta.mtime(),
            DateField::Created => meta.ctime(),
        }
        .map(|value| value.get() as i64)
    }

    fn ensure_metadata(&mut self, index: SlabIndex) -> SlabNodeMetadataCompact {
        let current = self.file_nodes[index].metadata;
        if current.is_some() {
            return current;
        }
        let path = self
            .node_path(index)
            .expect("node index is not present in slab");
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(data) => SlabNodeMetadataCompact::some(data.into()),
            Err(_) => SlabNodeMetadataCompact::unaccessible(),
        };
        self.file_nodes[index].metadata = metadata;
        metadata
    }
}

fn normalize_extensions(argument: &FilterArgument) -> HashSet<String> {
    let mut values = HashSet::new();
    match &argument.kind {
        ArgumentKind::List(list) => {
            for item in list {
                if let Some(ext) = normalize_extension(item) {
                    values.insert(ext);
                }
            }
        }
        _ => {
            if let Some(ext) = normalize_extension(&argument.raw) {
                values.insert(ext);
            }
        }
    }
    values
}

fn normalize_extension(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches('.');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_ascii_lowercase())
    }
}

fn extension_of(name: &str) -> Option<String> {
    let pos = name.rfind('.')?;
    if pos + 1 >= name.len() {
        return None;
    }
    Some(name[pos + 1..].to_ascii_lowercase())
}

#[derive(Clone, Copy)]
enum TypeFilterTarget {
    NodeType(NodeFileType),
    Extensions(&'static [&'static str]),
}

fn lookup_type_group(name: &str) -> Option<TypeFilterTarget> {
    match name {
        "file" | "files" => Some(TypeFilterTarget::NodeType(NodeFileType::File)),
        "folder" | "folders" | "dir" | "directory" => {
            Some(TypeFilterTarget::NodeType(NodeFileType::Dir))
        }
        "picture" | "pictures" | "image" | "images" | "photo" | "photos" => {
            Some(TypeFilterTarget::Extensions(PICTURE_EXTENSIONS))
        }
        "video" | "videos" | "movie" | "movies" => {
            Some(TypeFilterTarget::Extensions(VIDEO_EXTENSIONS))
        }
        "audio" | "audios" | "music" | "song" | "songs" => {
            Some(TypeFilterTarget::Extensions(AUDIO_EXTENSIONS))
        }
        "doc" | "docs" | "document" | "documents" | "text" | "office" => {
            Some(TypeFilterTarget::Extensions(DOCUMENT_EXTENSIONS))
        }
        "presentation" | "presentations" | "ppt" | "slides" => {
            Some(TypeFilterTarget::Extensions(PRESENTATION_EXTENSIONS))
        }
        "spreadsheet" | "spreadsheets" | "xls" | "excel" | "sheet" | "sheets" => {
            Some(TypeFilterTarget::Extensions(SPREADSHEET_EXTENSIONS))
        }
        "pdf" => Some(TypeFilterTarget::Extensions(PDF_EXTENSIONS)),
        "archive" | "archives" | "compressed" | "zip" => {
            Some(TypeFilterTarget::Extensions(ARCHIVE_EXTENSIONS))
        }
        "code" | "source" | "dev" => Some(TypeFilterTarget::Extensions(CODE_EXTENSIONS)),
        "exe" | "exec" | "executable" | "executables" | "program" | "programs" | "app" | "apps" => {
            Some(TypeFilterTarget::Extensions(EXECUTABLE_EXTENSIONS))
        }
        _ => None,
    }
}

const PICTURE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tif", "tiff", "webp", "ico", "svg", "heic", "heif", "raw",
    "arw", "cr2", "orf", "raf", "psd", "ai",
];
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "avi", "mkv", "wmv", "webm", "flv", "mpg", "mpeg", "3gp", "3g2", "ts",
    "mts", "m2ts",
];
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "ogg", "oga", "opus", "wma", "m4a", "alac", "aiff",
];
const DOCUMENT_EXTENSIONS: &[&str] = &[
    "txt", "md", "rst", "doc", "docx", "rtf", "odt", "pdf", "pages", "rtfd",
];
const PRESENTATION_EXTENSIONS: &[&str] = &["ppt", "pptx", "key", "odp"];
const SPREADSHEET_EXTENSIONS: &[&str] = &["xls", "xlsx", "csv", "numbers", "ods"];
const PDF_EXTENSIONS: &[&str] = &["pdf"];
const ARCHIVE_EXTENSIONS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz", "zst", "cab", "iso", "dmg",
];
const CODE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "c", "cc", "cpp", "cxx", "h", "hpp", "hh", "java", "cs", "py",
    "go", "rb", "swift", "kt", "kts", "php", "html", "css", "scss", "sass", "less", "json", "yaml",
    "yml", "toml", "ini", "cfg", "sh", "zsh", "fish", "ps1", "psm1", "sql", "lua", "pl", "pm", "r",
    "m", "mm", "dart", "scala", "ex", "exs",
];
const EXECUTABLE_EXTENSIONS: &[&str] = &[
    "exe", "msi", "bat", "cmd", "com", "ps1", "psm1", "app", "apk", "ipa", "jar", "bin", "run",
    "pkg",
];

#[derive(Clone, Copy)]
enum DateField {
    Modified,
    Created,
}

struct DateContext {
    tz: TimeZone,
    today: Date,
}

impl DateContext {
    fn capture() -> Self {
        let tz = TimeZone::system();
        let zoned = Timestamp::now().to_zoned(tz.clone());
        Self {
            tz,
            today: zoned.date(),
        }
    }
}

struct DatePredicate {
    kind: DatePredicateKind,
}

#[derive(Clone, Copy)]
enum DatePredicateKind {
    Range {
        start: Option<i64>,
        end: Option<i64>,
    },
    NotEqual {
        start: i64,
        end: i64,
    },
}

impl DatePredicate {
    fn parse(argument: &FilterArgument, context: &DateContext) -> Result<Self> {
        match &argument.kind {
            ArgumentKind::Range(range) => {
                let start = match &range.start {
                    Some(value) => Some(parse_date_value(value, context)?.start),
                    None => None,
                };
                let end = match &range.end {
                    Some(value) => Some(parse_date_value(value, context)?.end),
                    None => None,
                };
                if let (Some(s), Some(e)) = (start, end) {
                    if s > e {
                        bail!("date range start must not exceed end");
                    }
                }
                Ok(Self {
                    kind: DatePredicateKind::Range { start, end },
                })
            }
            ArgumentKind::Comparison(comp) => {
                let value = parse_date_value(&comp.value, context)?;
                let predicate = match comp.op {
                    ComparisonOp::Lt => {
                        let bound = value.start.saturating_sub(1);
                        DatePredicate::range(None, Some(bound))
                    }
                    ComparisonOp::Lte => DatePredicate::range(None, Some(value.end)),
                    ComparisonOp::Gt => DatePredicate::range(Some(value.end + 1), None),
                    ComparisonOp::Gte => DatePredicate::range(Some(value.start), None),
                    ComparisonOp::Eq => DatePredicate::range(Some(value.start), Some(value.end)),
                    ComparisonOp::Ne => DatePredicate {
                        kind: DatePredicateKind::NotEqual {
                            start: value.start,
                            end: value.end,
                        },
                    },
                };
                Ok(predicate)
            }
            ArgumentKind::Phrase | ArgumentKind::Bare => {
                let value = parse_date_value(&argument.raw, context)?;
                Ok(DatePredicate::range(Some(value.start), Some(value.end)))
            }
            ArgumentKind::List(_) => bail!("date filters do not accept lists"),
        }
    }

    fn range(start: Option<i64>, end: Option<i64>) -> Self {
        Self {
            kind: DatePredicateKind::Range { start, end },
        }
    }

    fn matches(&self, timestamp: i64) -> bool {
        match self.kind {
            DatePredicateKind::Range { start, end } => {
                if let Some(bound) = start {
                    if timestamp < bound {
                        return false;
                    }
                }
                if let Some(bound) = end {
                    if timestamp > bound {
                        return false;
                    }
                }
                true
            }
            DatePredicateKind::NotEqual { start, end } => timestamp < start || timestamp > end,
        }
    }
}

struct DateValue {
    start: i64,
    end: i64,
}

fn parse_date_value(raw: &str, context: &DateContext) -> Result<DateValue> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("date filters require a value");
    }
    if let Some(range) = keyword_range(trimmed, context) {
        return Ok(range);
    }
    if let Some(date) = parse_absolute_date(trimmed) {
        if let Some(bounds) = day_bounds(date, context) {
            return Ok(DateValue {
                start: bounds.0,
                end: bounds.1,
            });
        } else {
            bail!("Date {trimmed:?} is out of range");
        }
    }
    bail!("Unrecognized date literal: {trimmed}");
}

fn keyword_range(keyword: &str, context: &DateContext) -> Option<DateValue> {
    let lower = keyword.to_ascii_lowercase();
    let today = context.today;
    let year = today.year();
    let month = today.month();
    match lower.as_str() {
        "today" => day_bounds(today, context).map(|(s, e)| DateValue { start: s, end: e }),
        "yesterday" => {
            let date = shift_days(today, -1)?;
            day_bounds(date, context).map(|(s, e)| DateValue { start: s, end: e })
        }
        "thisweek" => {
            let weekday_offset = i64::from(today.weekday().to_monday_zero_offset());
            let start = shift_days(today, -weekday_offset)?;
            let end = shift_days(start, 6)?;
            range_from_dates(start, end, context)
        }
        "lastweek" => {
            let weekday_offset = i64::from(today.weekday().to_monday_zero_offset()) + 7;
            let start = shift_days(today, -weekday_offset)?;
            let end = shift_days(start, 6)?;
            range_from_dates(start, end, context)
        }
        "thismonth" => month_range(year, month, context),
        "lastmonth" => {
            let (year, month) = if month == 1 {
                (year.checked_sub(1)?, 12)
            } else {
                (year, month - 1)
            };
            month_range(year, month, context)
        }
        "thisyear" => year_range(year, context),
        "lastyear" => year_range(year.checked_sub(1)?, context),
        "pastweek" => trailing_range(context, 7),
        "pastmonth" => trailing_range(context, 30),
        "pastyear" => trailing_range(context, 365),
        _ => None,
    }
}

fn trailing_range(context: &DateContext, days: i64) -> Option<DateValue> {
    let start_date = shift_days(context.today, -days)?;
    range_from_dates(start_date, context.today, context)
}

fn month_range(year: i16, month: i8, context: &DateContext) -> Option<DateValue> {
    let start = Date::new(year, month, 1).ok()?;
    let (next_year, next_month) = if month == 12 {
        (year.checked_add(1)?, 1)
    } else {
        (year, month + 1)
    };
    let next_start = Date::new(next_year, next_month, 1).ok()?;
    let end = next_start.yesterday().ok()?;
    range_from_dates(start, end, context)
}

fn year_range(year: i16, context: &DateContext) -> Option<DateValue> {
    let start = Date::new(year, 1, 1).ok()?;
    let end = Date::new(year, 12, 31).ok()?;
    range_from_dates(start, end, context)
}

fn range_from_dates(start: Date, end: Date, context: &DateContext) -> Option<DateValue> {
    if end < start {
        return None;
    }
    let (start_ts, _) = day_bounds(start, context)?;
    let (_, end_ts) = day_bounds(end, context)?;
    Some(DateValue {
        start: start_ts,
        end: end_ts,
    })
}

fn shift_days(date: Date, delta: i64) -> Option<Date> {
    if delta == 0 {
        return Some(date);
    }
    let mut current = date;
    if delta > 0 {
        let steps = delta.unsigned_abs() as usize;
        for _ in 0..steps {
            current = current.tomorrow().ok()?;
        }
    } else {
        let steps = (-delta).unsigned_abs() as usize;
        for _ in 0..steps {
            current = current.yesterday().ok()?;
        }
    }
    Some(current)
}

fn day_bounds(date: Date, context: &DateContext) -> Option<(i64, i64)> {
    let start = context
        .tz
        .to_zoned(date.at(0, 0, 0, 0))
        .ok()?
        .timestamp()
        .as_second();
    let next_day = date.tomorrow().ok()?;
    let next_start = context
        .tz
        .to_zoned(next_day.at(0, 0, 0, 0))
        .ok()?
        .timestamp()
        .as_second();
    let end = next_start.checked_sub(1)?;
    Some((start, end))
}

fn parse_absolute_date(raw: &str) -> Option<Date> {
    let trimmed = raw.trim();
    let sep = trimmed.chars().find(|ch| matches!(ch, '-' | '/' | '.'))?;
    let mut formats = match sep {
        '-' => vec!["%Y-%m-%d", "%d-%m-%Y", "%m-%d-%Y"],
        '/' => vec!["%Y/%m/%d", "%m/%d/%Y", "%d/%m/%Y"],
        '.' => vec!["%Y.%m.%d", "%d.%m.%Y", "%m.%d.%Y"],
        _ => vec![],
    };
    let starts_with_year = trimmed.len() >= 4
        && trimmed.chars().take(4).all(|c| c.is_ascii_digit())
        && matches!(trimmed.chars().nth(4), Some('-' | '/' | '.'));
    formats.sort_by_key(|fmt| {
        let year_first = fmt.starts_with("%Y");
        if starts_with_year {
            if year_first { 0 } else { 1 }
        } else if year_first {
            1
        } else {
            0
        }
    });
    for fmt in formats {
        if let Ok(date) = Date::strptime(fmt, trimmed) {
            return Some(date);
        }
    }
    None
}

struct SizePredicate {
    kind: SizePredicateKind,
}

enum SizePredicateKind {
    Comparison { op: ComparisonOp, value: u64 },
    Range { min: Option<u64>, max: Option<u64> },
}

impl SizePredicate {
    fn parse(argument: &FilterArgument) -> Result<Self> {
        match &argument.kind {
            ArgumentKind::Comparison(comp) => {
                if size_keyword(&comp.value).is_some() {
                    bail!("size keywords cannot be used with comparison operators");
                }
                let value = parse_size_literal(&comp.value)?;
                Ok(SizePredicate {
                    kind: SizePredicateKind::Comparison { op: comp.op, value },
                })
            }
            ArgumentKind::Range(range) => {
                if range.separator != RangeSeparator::Dots {
                    bail!("size: only .. ranges are supported");
                }
                let start = match &range.start {
                    Some(value) => Some(parse_size_literal(value)?),
                    None => None,
                };
                let end = match &range.end {
                    Some(value) => Some(parse_size_literal(value)?),
                    None => None,
                };
                if let (Some(s), Some(e)) = (start, end) {
                    if s > e {
                        bail!("size range start must be less than or equal to the end");
                    }
                }
                Ok(SizePredicate {
                    kind: SizePredicateKind::Range {
                        min: start,
                        max: end,
                    },
                })
            }
            ArgumentKind::List(_) => bail!("size: lists are not supported"),
            _ => SizePredicate::from_bare_value(&argument.raw),
        }
    }

    fn from_bare_value(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!("size: requires a value");
        }
        if let Some(range) = size_keyword(trimmed) {
            return Ok(SizePredicate {
                kind: SizePredicateKind::Range {
                    min: range.min,
                    max: range.max,
                },
            });
        }
        let value = parse_size_literal(trimmed)?;
        Ok(SizePredicate {
            kind: SizePredicateKind::Comparison {
                op: ComparisonOp::Eq,
                value,
            },
        })
    }

    fn matches(&self, size: u64) -> bool {
        match &self.kind {
            SizePredicateKind::Comparison { op, value } => match op {
                ComparisonOp::Lt => size < *value,
                ComparisonOp::Lte => size <= *value,
                ComparisonOp::Gt => size > *value,
                ComparisonOp::Gte => size >= *value,
                ComparisonOp::Eq => size == *value,
                ComparisonOp::Ne => size != *value,
            },
            SizePredicateKind::Range { min, max } => {
                if let Some(start) = min {
                    if size < *start {
                        return false;
                    }
                }
                if let Some(end) = max {
                    if size > *end {
                        return false;
                    }
                }
                true
            }
        }
    }
}

struct SizeKeywordRange {
    min: Option<u64>,
    max: Option<u64>,
}

fn size_keyword(name: &str) -> Option<SizeKeywordRange> {
    let normalized = name.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "empty" => Some(SizeKeywordRange {
            min: Some(0),
            max: Some(0),
        }),
        "tiny" => Some(SizeKeywordRange {
            min: Some(0),
            max: Some(10 * KB),
        }),
        "small" => Some(SizeKeywordRange {
            min: Some(10 * KB + 1),
            max: Some(100 * KB),
        }),
        "medium" => Some(SizeKeywordRange {
            min: Some(100 * KB + 1),
            max: Some(MB),
        }),
        "large" => Some(SizeKeywordRange {
            min: Some(MB + 1),
            max: Some(16 * MB),
        }),
        "huge" => Some(SizeKeywordRange {
            min: Some(16 * MB + 1),
            max: Some(128 * MB),
        }),
        "gigantic" | "giant" => Some(SizeKeywordRange {
            min: Some(128 * MB + 1),
            max: None,
        }),
        _ => None,
    }
}

const KB: u64 = 1024;
const MB: u64 = 1024 * 1024;

fn parse_size_literal(raw: &str) -> Result<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("size: expected a number");
    }
    let mut split = trimmed.len();
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() || ch == '.' {
            continue;
        }
        split = idx;
        break;
    }

    let (value_part, unit_part) = trimmed.split_at(split);
    if value_part.is_empty() {
        bail!("size: expected a numeric value in {raw:?}");
    }
    let value: f64 = value_part
        .parse()
        .map_err(|_| anyhow!("size: failed to parse number in {raw:?}"))?;
    let multiplier = size_unit_multiplier(unit_part)?;
    let bytes = (value * multiplier as f64).round();
    if !bytes.is_finite() || bytes < 0.0 {
        bail!("size: value {raw:?} is out of range");
    }
    if bytes > u64::MAX as f64 {
        Ok(u64::MAX)
    } else {
        Ok(bytes as u64)
    }
}

fn size_unit_multiplier(unit: &str) -> Result<u64> {
    let normalized = unit.trim().to_ascii_lowercase();
    let multiplier = match normalized.as_str() {
        "" | "b" | "byte" | "bytes" => 1,
        "k" | "kb" | "kib" | "kilobyte" | "kilobytes" => 1024,
        "m" | "mb" | "mib" | "megabyte" | "megabytes" => 1024 * 1024,
        "g" | "gb" | "gib" | "gigabyte" | "gigabytes" => 1024 * 1024 * 1024,
        "t" | "tb" | "tib" | "terabyte" | "terabytes" => 1024_u64.pow(4),
        "p" | "pb" | "pib" | "petabyte" | "petabytes" => 1024_u64.pow(5),
        _ => bail!("Unknown size unit: {unit:?}"),
    };
    Ok(multiplier)
}

fn filter_nodes(
    nodes: Vec<SlabIndex>,
    token: CancellationToken,
    mut predicate: impl FnMut(SlabIndex) -> bool,
) -> Option<Vec<SlabIndex>> {
    let mut filtered = Vec::with_capacity(nodes.len());
    let mut counter = 0usize;
    for index in nodes {
        // While filtering dc: dm:, lstat is slow. Thus we check cancellation more frequently.
        token.is_cancelled_sparse(counter)?;
        counter = counter.wrapping_add(4);
        if predicate(index) {
            filtered.push(index);
        }
    }
    Some(filtered)
}

fn intersect_in_place(
    values: &mut Vec<SlabIndex>,
    rhs: &[SlabIndex],
    token: CancellationToken,
) -> Option<()> {
    if values.is_empty() {
        return Some(());
    }
    let rhs_set: HashSet<SlabIndex> = rhs.iter().copied().collect();
    let mut filtered = Vec::with_capacity(values.len().min(rhs.len()));
    for (i, index) in values.iter().copied().enumerate() {
        token.is_cancelled_sparse(i)?;
        if rhs_set.contains(&index) {
            filtered.push(index);
        }
    }
    *values = filtered;
    Some(())
}

fn difference_in_place(
    values: &mut Vec<SlabIndex>,
    rhs: &[SlabIndex],
    token: CancellationToken,
) -> Option<()> {
    if values.is_empty() || rhs.is_empty() {
        return Some(());
    }
    let rhs_set: HashSet<SlabIndex> = rhs.iter().copied().collect();
    let mut filtered = Vec::with_capacity(values.len());
    for (i, index) in values.iter().copied().enumerate() {
        token.is_cancelled_sparse(i)?;
        if !rhs_set.contains(&index) {
            filtered.push(index);
        }
    }
    *values = filtered;
    Some(())
}

fn union_in_place(
    values: &mut Vec<SlabIndex>,
    rhs: &[SlabIndex],
    token: CancellationToken,
) -> Option<()> {
    if rhs.is_empty() {
        return Some(());
    }
    let mut seen: HashSet<SlabIndex> = values.iter().copied().collect();
    for (i, index) in rhs.iter().copied().enumerate() {
        token.is_cancelled_sparse(i)?;
        if seen.insert(index) {
            values.push(index);
        }
    }
    Some(())
}
