export const APP_TOOLTIP_ID = 'cardinal-app-tooltip';

export const getTooltipAttributes = (content?: string | null): Record<string, string> => {
  if (!content) {
    return {};
  }

  return {
    'data-tooltip-id': APP_TOOLTIP_ID,
    'data-tooltip-content': content,
  };
};
