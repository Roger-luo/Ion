export interface NavItem {
  title: string;
  slug: string;
}

export interface NavSection {
  title: string;
  items: NavItem[];
}

export const navigation: NavSection[] = [
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
      { title: 'Adding Skills', slug: 'guides/adding-skills' },
      { title: 'Configuration', slug: 'guides/configuration' },
    ],
  },
];

/** Flatten all navigation sections into a single ordered list of NavItems. */
export function getFlatNavItems(): NavItem[] {
  return navigation.flatMap((section) => section.items);
}

/** Return the previous and next NavItems relative to the given slug. */
export function getPrevNext(currentSlug: string): {
  prev: NavItem | null;
  next: NavItem | null;
} {
  const flat = getFlatNavItems();
  const index = flat.findIndex((item) => item.slug === currentSlug);

  if (index === -1) {
    return { prev: null, next: null };
  }

  return {
    prev: index > 0 ? flat[index - 1] : null,
    next: index < flat.length - 1 ? flat[index + 1] : null,
  };
}
