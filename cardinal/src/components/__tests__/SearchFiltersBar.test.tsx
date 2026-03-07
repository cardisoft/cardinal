import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SearchFiltersBar } from '../SearchFiltersBar';

const pickFolderMock = vi.fn();

vi.mock('../../utils/pickFolder', () => ({
  pickFolder: () => pickFolderMock(),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe('SearchFiltersBar', () => {
  it('sends extension selections as merge actions', async () => {
    const user = userEvent.setup();
    const onApplyAction = vi.fn();
    const onOpenHelp = vi.fn();

    render(
      <SearchFiltersBar
        onApplyAction={onApplyAction}
        onOpenHelp={onOpenHelp}
        helpButtonLabel="Show search help"
      />,
    );

    await user.selectOptions(
      screen.getByRole('combobox', { name: 'search.filterBar.extension.label' }),
      'pdf',
    );

    expect(onApplyAction).toHaveBeenCalledWith({
      kind: 'merge-extension',
      extension: 'pdf',
    });
  });

  it('replaces single-choice filter families from dropdowns', async () => {
    const user = userEvent.setup();
    const onApplyAction = vi.fn();
    const onOpenHelp = vi.fn();

    render(
      <SearchFiltersBar
        onApplyAction={onApplyAction}
        onOpenHelp={onOpenHelp}
        helpButtonLabel="Show search help"
      />,
    );

    await user.selectOptions(
      screen.getByRole('combobox', { name: 'search.filterBar.type.label' }),
      'type:code',
    );

    expect(onApplyAction).toHaveBeenCalledWith({
      kind: 'replace-family',
      family: 'type',
      token: 'type:code',
    });
  });

  it('quotes picked folder paths when applying scope filters', async () => {
    const user = userEvent.setup();
    const onApplyAction = vi.fn();
    const onOpenHelp = vi.fn();
    pickFolderMock.mockResolvedValue('/Users/demo/Application Support');

    render(
      <SearchFiltersBar
        onApplyAction={onApplyAction}
        onOpenHelp={onOpenHelp}
        helpButtonLabel="Show search help"
      />,
    );

    await user.click(screen.getByRole('button', { name: 'search.filterBar.scope.in' }));

    await waitFor(() => {
      expect(onApplyAction).toHaveBeenCalledWith({
        kind: 'replace-family',
        family: 'scope',
        token: 'in:"/Users/demo/Application Support"',
      });
    });
  });

  it('opens search help from the trailing info button', async () => {
    const user = userEvent.setup();
    const onApplyAction = vi.fn();
    const onOpenHelp = vi.fn();

    render(
      <SearchFiltersBar
        onApplyAction={onApplyAction}
        onOpenHelp={onOpenHelp}
        helpButtonLabel="Show search help"
      />,
    );

    await user.click(screen.getByRole('button', { name: 'Show search help' }));

    expect(onOpenHelp).toHaveBeenCalledTimes(1);
  });

  it('adds tooltip descriptions to filter controls', () => {
    const onApplyAction = vi.fn();
    const onOpenHelp = vi.fn();

    render(
      <SearchFiltersBar
        onApplyAction={onApplyAction}
        onOpenHelp={onOpenHelp}
        helpButtonLabel="Show search help"
      />,
    );

    expect(screen.getByRole('button', { name: 'search.filterBar.scope.in' })).toHaveAttribute(
      'data-tooltip-content',
      'search.filterBar.tooltips.scope.in',
    );
    expect(screen.getByRole('combobox', { name: 'search.filterBar.type.label' })).toHaveAttribute(
      'data-tooltip-content',
      'search.filterBar.tooltips.type',
    );
  });
});
