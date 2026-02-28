/**
 * Utility to manipulate colors
 */

/**
 * Changes the alpha (opacity) of an rgba color string or hex color
 * @param color The color string (e.g., "rgba(0, 0, 0, 0.1)" or "#000000")
 * @param newAlpha The new alpha value (e.g., 1.0)
 * @returns The modified rgba string
 */
export const setAlpha = (color: string | undefined, newAlpha: number | string): string => {
  if (!color) return '';
  
  if (color.startsWith('#')) {
    const r = parseInt(color.slice(1, 3), 16);
    const g = parseInt(color.slice(3, 5), 16);
    const b = parseInt(color.slice(5, 7), 16);
    return `rgba(${r}, ${g}, ${b}, ${newAlpha})`;
  }
  
  if (!color.startsWith('rgba')) {
    return color;
  }
  return color.replace(/[\d.]+\)$/g, `${newAlpha})`);
};

/**
 * Ensures a color is solid (alpha 1.0)
 */
export const toSolidColor = (rgba: string | undefined): string => {
  return setAlpha(rgba, 1.0);
};
