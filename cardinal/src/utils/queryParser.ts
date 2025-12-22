import { invoke } from '@tauri-apps/api/core';
import type { FilterRule, RuleGroup, SearchField } from '../components/QueryBuilder';

// Types matching Rust serialization
interface ParsedExpr {
  type: 'empty' | 'term' | 'not' | 'and' | 'or';
  term?: ParsedTerm;
  inner?: ParsedExpr;
  parts?: ParsedExpr[];
}

interface ParsedTerm {
  type: 'word' | 'phrase' | 'regex' | 'filter';
  text?: string;
  pattern?: string;
  kind?: string;
  argument?: string | null;
}

export async function parseQueryToBuilder(queryText: string): Promise<RuleGroup | null> {
  if (!queryText.trim()) {
    return {
      id: 'root',
      type: 'group',
      combinator: 'and',
      items: [],
      exclude: false,
    };
  }

  try {
    const parsed = await invoke<ParsedExpr>('parse_search_query', { query: queryText });
    console.log('Parsed query:', queryText);
    console.log('Raw parsed:', JSON.stringify(parsed, null, 2));
    const result = exprToGroup(parsed, true);
    console.log('Converted to builder:', result);
    return result;
  } catch (error) {
    console.error('Failed to parse query:', error);
    return null;
  }
}

function exprToGroup(expr: ParsedExpr, isRoot = false): RuleGroup {
  const id = isRoot ? 'root' : crypto.randomUUID();

  switch (expr.type) {
    case 'empty':
      return {
        id,
        type: 'group',
        combinator: 'and',
        items: [],
        exclude: false,
      };

    case 'term':
      if (!expr.term) {
        return {
          id,
          type: 'group',
          combinator: 'and',
          items: [],
          exclude: false,
        };
      }
      const rule = termToRule(expr.term);
      return {
        id,
        type: 'group',
        combinator: 'and',
        items: rule ? [rule] : [],
        exclude: false,
      };

    case 'not':
      if (!expr.inner) {
        return {
          id,
          type: 'group',
          combinator: 'and',
          items: [],
          exclude: false,
        };
      }
      // If NOT wraps a single term, convert to excluded rule
      if (expr.inner.type === 'term' && expr.inner.term) {
        const rule = termToRule(expr.inner.term);
        if (rule) {
          rule.exclude = true;
          return {
            id,
            type: 'group',
            combinator: 'and',
            items: [rule],
            exclude: false,
          };
        }
      }
      // Otherwise, negate the entire group
      const innerGroup = exprToGroup(expr.inner);
      innerGroup.exclude = true;
      return isRoot
        ? innerGroup
        : {
            id,
            type: 'group',
            combinator: 'and',
            items: [innerGroup],
            exclude: false,
          };

    case 'and':
      return {
        id,
        type: 'group',
        combinator: 'and',
        items: (expr.parts || [])
          .map((part) => {
            if (part.type === 'term' && part.term) {
              const rule = termToRule(part.term);
              return rule || exprToGroup(part);
            }
            return exprToGroup(part);
          })
          .filter(Boolean),
        exclude: false,
      };

    case 'or':
      return {
        id,
        type: 'group',
        combinator: 'or',
        items: (expr.parts || [])
          .map((part) => {
            if (part.type === 'term' && part.term) {
              const rule = termToRule(part.term);
              return rule || exprToGroup(part);
            }
            return exprToGroup(part);
          })
          .filter(Boolean),
        exclude: false,
      };

    default:
      return {
        id,
        type: 'group',
        combinator: 'and',
        items: [],
        exclude: false,
      };
  }
}

function termToRule(term: ParsedTerm): FilterRule | null {
  const id = crypto.randomUUID();

  switch (term.type) {
    case 'word':
    case 'phrase':
      return {
        id,
        type: 'rule',
        field: 'name',
        operator: 'contains',
        value: term.text || '',
        exclude: false,
      };

    case 'filter':
      if (!term.kind) return null;

      // Map filter kind to SearchField
      const fieldMap: Record<string, SearchField> = {
        file: 'kind',
        folder: 'kind',
        ext: 'ext',
        type: 'type',
        size: 'size',
        dm: 'dm',
        dc: 'dc',
        parent: 'parent',
        infolder: 'infolder',
        nosubfolders: 'nosubfolders',
        content: 'content',
        tag: 'tag',
        regex: 'regex',
      };

      const field = fieldMap[term.kind];
      if (!field) return null;

      // Special handling for file/folder
      let value = term.argument || '';
      if (term.kind === 'file' || term.kind === 'folder') {
        value = term.kind;
      }

      return {
        id,
        type: 'rule',
        field,
        operator: 'contains',
        value,
        exclude: false,
      };

    case 'regex':
      return {
        id,
        type: 'rule',
        field: 'regex',
        operator: 'contains',
        value: term.pattern || '',
        exclude: false,
      };

    default:
      return null;
  }
}
