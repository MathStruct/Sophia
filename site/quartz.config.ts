import { QuartzConfig } from "./quartz/cfg"
import * as Plugin from "./quartz/plugins"

/**
 * Quartz 4 Configuration
 *
 * See https://quartz.jzhao.xyz/configuration for more information.
 *
 * The content directory is the repository root (`npx quartz build -d ..`):
 * the whole repository is the Obsidian vault, so the per-file design notes
 * next to the Rust and Julia sources are published alongside `markdown/`, and
 * `[[wikilinks]]` between them resolve exactly as they do in Obsidian.
 */
const config: QuartzConfig = {
  configuration: {
    pageTitle: "Sophia",
    pageTitleSuffix: "",
    enableSPA: true,
    enablePopovers: true,
    analytics: null,
    locale: "en-US",
    // project page of the MathStruct organisation, whose user site carries the
    // custom domain; GitHub serves it under that domain as well
    baseUrl: "mathstruct.org/Sophia",
    ignorePatterns: [
      // the site generator itself lives inside the vault
      "site",
      // Obsidian's own state and the inline-tikz plugin's cache
      ".obsidian",
      ".tikz-cache",
      // repository documentation; the site has its own landing page (index.md)
      "README.md",
      // prompts used while drafting the vault, not part of the design
      "markdown/Prompts",
      // sources and manifests: every .rs/.jl has a .md sibling that *is* published
      "**/*.rs",
      "**/*.jl",
      "**/*.toml",
      "Cargo.lock",
      "target",
      "private",
      "templates",
    ],
    // the notes are a design document, not a blog: date them by last edit
    defaultDateType: "modified",
    theme: {
      fontOrigin: "googleFonts",
      cdnCaching: true,
      typography: {
        header: "Schibsted Grotesk",
        body: "Source Sans Pro",
        code: "IBM Plex Mono",
      },
      colors: {
        lightMode: {
          light: "#faf8f8",
          lightgray: "#e5e5e5",
          gray: "#b8b8b8",
          darkgray: "#4e4e4e",
          dark: "#2b2b2b",
          secondary: "#284b63",
          tertiary: "#84a59d",
          highlight: "rgba(143, 159, 169, 0.15)",
          textHighlight: "#fff23688",
        },
        darkMode: {
          light: "#161618",
          lightgray: "#393639",
          gray: "#646464",
          darkgray: "#d4d4d4",
          dark: "#ebebec",
          secondary: "#7b97aa",
          tertiary: "#84a59d",
          highlight: "rgba(143, 159, 169, 0.15)",
          textHighlight: "#b3aa0288",
        },
      },
    },
  },
  plugins: {
    transformers: [
      Plugin.FrontMatter(),
      Plugin.CreatedModifiedDate({
        priority: ["frontmatter", "git", "filesystem"],
      }),
      // notes open with `# <note name>`, which Quartz already shows as the title
      Plugin.DropTitleHeading(),
      Plugin.SyntaxHighlighting({
        theme: {
          light: "github-light",
          dark: "github-dark",
        },
        keepBackground: false,
      }),
      // ```tikz fences in the inline-tikz format, compiled to SVG at build
      // time and cached by content hash in site/.tikz-cache (committed)
      Plugin.TikZ({ cacheDir: ".tikz-cache" }),
      Plugin.ObsidianTabs(),
      Plugin.ObsidianFlavoredMarkdown({ enableInHtmlEmbed: false }),
      Plugin.GitHubFlavoredMarkdown(),
      Plugin.TableOfContents(),
      Plugin.CrawlLinks({ markdownLinkResolution: "shortest" }),
      Plugin.Description(),
      // Formulas are written in Typst (the vault uses wypst). Each one is
      // offered to Typst first; anything Typst rejects falls through to KaTeX,
      // so a stray LaTeX formula still renders. Pin a page with `math: typst`
      // or `math: latex` to forbid the other syntax.
      Plugin.Latex({
        renderEngine: ["typst", "katex"],
        katexOptions: { throwOnError: false, strict: false },
      }),
    ],
    filters: [Plugin.RemoveDrafts()],
    emitters: [
      Plugin.AliasRedirects(),
      Plugin.ComponentResources(),
      Plugin.ContentPage(),
      Plugin.FolderPage(),
      Plugin.TagPage(),
      Plugin.ContentIndex({
        enableSiteMap: true,
        enableRSS: true,
      }),
      Plugin.Assets(),
      Plugin.Static(),
      Plugin.Favicon(),
      Plugin.NotFoundPage(),
      // Plugin.CustomOgImages(),  // disabled: slows the build considerably
    ],
  },
}

export default config
