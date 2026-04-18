import apiNav from './api-navigation.json';

export interface NavItem {
  title: string;
  slug: string;
}

export interface NavSection {
  title: string;
  items: NavItem[];
}

export const mainNavigation: NavSection[] = [
  {
    title: 'Getting Started',
    items: [
      { title: 'Introduction', slug: 'getting-started/introduction' },
      { title: 'Installation', slug: 'getting-started/installation' },
      { title: 'Quick Start', slug: 'getting-started/quick-start' },
    ],
  },
  {
    title: 'Guides',
    items: [
      { title: 'Typical Workflows', slug: 'guides/workflows' },
      { title: 'Binary Skills', slug: 'guides/binary-skills' },
      { title: 'Adding Skills', slug: 'guides/adding-skills' },
      { title: 'Configuration', slug: 'guides/configuration' },
    ],
  },
];

export const apiNavigation: NavSection[] = apiNav;

/** All navigation sections combined (used for search/sitemap). */
export const navigation: NavSection[] = [...mainNavigation, ...apiNavigation];

/** Return the navigation sections relevant to a given slug. */
export function getNavigationForSlug(slug: string): NavSection[] {
  return slug.startsWith('api-reference/') ? apiNavigation : mainNavigation;
}

function flatItems(sections: NavSection[]): NavItem[] {
  return sections.flatMap((s) => s.items);
}

export function getFlatNavItems(): NavItem[] {
  return flatItems(navigation);
}

/** Return prev/next within the same section (docs or API). */
export function getPrevNext(currentSlug: string): {
  prev: NavItem | null;
  next: NavItem | null;
} {
  const flat = flatItems(getNavigationForSlug(currentSlug));
  const index = flat.findIndex((item) => item.slug === currentSlug);

  if (index === -1) {
    return { prev: null, next: null };
  }

  return {
    prev: index > 0 ? flat[index - 1] : null,
    next: index < flat.length - 1 ? flat[index + 1] : null,
  };
}
