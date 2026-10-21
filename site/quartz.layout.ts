import { siteLayout } from "./mathstruct/layout"

export const { sharedPageComponents, defaultContentPageLayout, defaultListPageLayout } =
  siteLayout({
    footerLinks: {
      "Source on GitHub": "https://github.com/MathStruct/Sophia",
      MathStruct: "https://mathstruct.org/",
    },
    // The vault root is the repository root, so the explorer's top level is `markdown` (the
    // notes), `src` (Julia) and `crates` (Rust). Show them in that order -- prose first, then
    // the per-file notes -- instead of alphabetically. This function is serialised with
    // `toString()` and evaluated in the browser, so it must not close over anything defined
    // outside it, and must not define inner functions (esbuild wraps those in a `__name` helper
    // that is absent at runtime).
    explorer: {
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
    },
  })
