import { describe, expect, it } from 'vitest';
import { applySearchToolbarQueryAction } from '../searchToolbarQuery';

describe('applySearchToolbarQueryAction', () => {
  it('appends a token to the end of the query', () => {
    expect(
      applySearchToolbarQueryAction('report draft', {
        kind: 'append-token',
        token: 'regex:^README\\.md$',
      }),
    ).toBe('report draft regex:^README\\.md$');
  });

  it('merges extensions into a single ext token while keeping the first ext position', () => {
    expect(
      applySearchToolbarQueryAction('report ext:pdf notes ext:md', {
        kind: 'merge-extension',
        extension: 'rs',
      }),
    ).toBe('report ext:pdf;md;rs notes');
  });

  it('replaces type filters in place while removing previous top-level type tokens', () => {
    expect(
      applySearchToolbarQueryAction('report type:code audio:podcast size:small', {
        kind: 'replace-family',
        family: 'type',
        token: 'type:picture',
      }),
    ).toBe('report type:picture size:small');
  });

  it('replaces date filters across modified and created families in place', () => {
    expect(
      applySearchToolbarQueryAction('report dm:today size:huge dc:lastmonth', {
        kind: 'replace-family',
        family: 'date',
        token: 'dc:thisweek',
      }),
    ).toBe('report dc:thisweek size:huge');
  });

  it('keeps unrelated filters in place when replacing another family', () => {
    expect(
      applySearchToolbarQueryAction('report dm:thisweek size:huge', {
        kind: 'replace-family',
        family: 'size',
        token: 'size:small',
      }),
    ).toBe('report dm:thisweek size:small');
  });

  it('does not rewrite grouped expressions while still appending the new family token', () => {
    expect(
      applySearchToolbarQueryAction('report (type:code | type:doc)', {
        kind: 'replace-family',
        family: 'type',
        token: 'type:picture',
      }),
    ).toBe('report (type:code | type:doc) type:picture');
  });
});
