// One job: a project's slug, derived from its name.

/** Lowercase ASCII letters and digits, runs of anything else as one dash, no
 * dash at either end. Accents are dropped rather than the letters they sit on. */
export function slugify(name: string): string {
  return name
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}
