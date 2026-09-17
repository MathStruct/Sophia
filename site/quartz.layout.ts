import { PageLayout, SharedLayout } from "./quartz/cfg"
import * as Component from "./quartz/components"

// components shared across all pages
export const sharedPageComponents: SharedLayout = {
  head: Component.Head(),
  header: [],
  afterBody: [],
  footer: Component.Footer({
    links: {
      "Source on GitHub": "https://github.com/MathStruct/Sophia",
      MathStruct: "https://mathstruct.org/",
    },
  }),
}

// The vault root is the repository root, so the explorer's top level is
// `markdown` (the notes), `src` (Julia) and `crates` (Rust). Show them in that
// order -- prose first, then the per-file notes -- instead of alphabetically.
// This function is serialised with `toString()` and evaluated in the browser,
// so it must not close over anything defined outside it, and must not define
// inner functions (esbuild wraps those in a `__name` helper that is absent at
// runtime).
const explorer = Component.Explorer({
  sortFn: (a, b) => {
    if (a.isFolder && b.isFolder) {
      const rank: Record<string, number> = { markdown: 0, src: 1, crates: 2 }
      const byRank = (rank[a.slugSegment] ?? 3) - (rank[b.slugSegment] ?? 3)
      if (byRank !== 0) return byRank
    }
    if (a.isFolder !== b.isFolder) return a.isFolder ? -1 : 1
    return a.displayName.localeCompare(b.displayName, undefined, {
      numeric: true,
      sensitivity: "base",
    })
  },
})

// components for pages that display a single page (e.g. a single note)
export const defaultContentPageLayout: PageLayout = {
  beforeBody: [
    Component.ConditionalRender({
      component: Component.Breadcrumbs(),
      condition: (page) => page.fileData.slug !== "index",
    }),
    Component.ArticleTitle(),
    // the landing page is not an article: no date or reading time on it
    Component.ConditionalRender({
      component: Component.ContentMeta(),
      condition: (page) => page.fileData.slug !== "index",
    }),
    Component.TagList(),
  ],
  left: [
    Component.PageTitle(),
    Component.MobileOnly(Component.Spacer()),
    Component.Flex({
      components: [
        {
          Component: Component.Search(),
          grow: true,
        },
        { Component: Component.Darkmode() },
        { Component: Component.ReaderMode() },
      ],
    }),
    explorer,
  ],
  right: [
    Component.Graph(),
    Component.DesktopOnly(Component.TableOfContents()),
    Component.Backlinks(),
  ],
}

// components for pages that display lists of pages  (e.g. tags or folders)
export const defaultListPageLayout: PageLayout = {
  beforeBody: [Component.Breadcrumbs(), Component.ArticleTitle(), Component.ContentMeta()],
  left: [
    Component.PageTitle(),
    Component.MobileOnly(Component.Spacer()),
    Component.Flex({
      components: [
        {
          Component: Component.Search(),
          grow: true,
        },
        { Component: Component.Darkmode() },
      ],
    }),
    explorer,
  ],
  right: [],
}
