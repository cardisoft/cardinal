import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SearchHelpOverlay } from '../SearchHelpOverlay';

const openUrlMock = vi.fn();
const onApplyExampleMock = vi.fn();
const onCloseMock = vi.fn();

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: (...args: unknown[]) => openUrlMock(...args),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe('SearchHelpOverlay', () => {
  it('opens the GitHub docs link', async () => {
    render(<SearchHelpOverlay open onClose={onCloseMock} onApplyExample={onApplyExampleMock} />);

    fireEvent.click(screen.getByRole('link', { name: 'search.help.openDocs' }));

    expect(openUrlMock).toHaveBeenCalledWith(
      'https://github.com/cardisoft/cardinal/blob/master/doc/pub/search-syntax.md',
    );
  });

  it('applies an example to the files search when clicked', () => {
    render(<SearchHelpOverlay open onClose={onCloseMock} onApplyExample={onApplyExampleMock} />);

    fireEvent.click(screen.getByRole('button', { name: 'test report' }));

    expect(onApplyExampleMock).toHaveBeenCalledWith('test report');
    expect(onCloseMock).not.toHaveBeenCalled();
  });

  it('shows a descriptive tooltip for each example', () => {
    render(<SearchHelpOverlay open onClose={onCloseMock} onApplyExample={onApplyExampleMock} />);

    expect(screen.getByRole('button', { name: 'test report' })).toHaveAttribute(
      'data-tooltip-content',
      'search.help.exampleDescriptions.plainWord',
    );
  });
});
