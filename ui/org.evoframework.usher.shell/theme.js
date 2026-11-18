// The theme the shell is drawn in. A theme is an evo artefact: a directory
// under the theme's name, with a stylesheet that sets its tokens as custom
// properties. The shell is drawn in the theme the steward says is active,
// and in the usher's own while none is.

/** The usher's own theme. */
export const OWN_THEME = "org.evoframework.usher.theme.frost";

/** A plugin's name, as evo writes one. */
const NAME = /^[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)+$/;

/**
 * Where the stylesheet of a theme is: the theme named, or the usher's own
 * where none is named, or what is named is no plugin's name.
 *
 * @param {unknown} theme the active theme, as the steward names it
 * @returns {string}
 */
export function themeAddress(theme) {
  const name = typeof theme === "string" && NAME.test(theme) ? theme : OWN_THEME;
  return `/${name}/theme.css`;
}
