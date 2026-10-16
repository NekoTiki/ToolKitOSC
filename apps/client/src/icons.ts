import { addCollection } from '@iconify/vue'

// Icons Lucide doesn't have, used as `i-tkosc-<name>` like any other UIcon name. Drawn in Lucide's
// style (24 px, stroke 2, round caps) so they sit with the rest.
//
// collar: IconMind's collar-outline-regular, https://github.com/Iconmind/iconmind
// MIT License, Copyright (c) 2026 IconMind contributors.
addCollection({
  prefix: 'tkosc',
  width: 24,
  height: 24,
  icons: {
    collar: {
      body: '<g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M3 6c0 6 4 9 9 9s9 -3 9 -9"/><path d="M10 18a2 2 0 1 0 4 0 2 2 0 1 0 -4 0"/></g>'
    }
  }
})
