export type FontSizeSettings = Readonly<{
  // Base UI font size in px; everything else (rows, dialogs, buttons) derives from it.
  base: number;
}>;

export const FONT_SIZE_MIN = 11;
export const FONT_SIZE_MAX = 20;
export const FONT_SIZE_DEFAULT = 13;

export const clampFontSize = (value: number): number =>
  Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, Math.round(value)));

/**
 * Row height tracks the base font size so the virtual list stays legible at larger sizes.
 * Keep in sync with the `.row` line-height in App.css.
 */
export const rowHeightForFontSize = (base: number): number =>
  Math.max(20, Math.round(clampFontSize(base) * 1.85));
