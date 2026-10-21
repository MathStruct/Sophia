import { siteConfig } from "./mathstruct/config"

/**
 * The plugins, theme and fixes shared by all MathStruct sites are in the preset
 * (https://github.com/MathStruct/quartz, mathstruct/config.ts); this file only says what is
 * particular to Sophia.
 *
 * The content directory is the repository root (see build.sh): the whole repository is the
 * Obsidian vault, so the per-file design notes next to the Rust and Julia sources are
 * published alongside `vault/`, and `[[wikilinks]]` between them resolve exactly as they do in
 * Obsidian.
 */
export default siteConfig({
  pageTitle: "Sophia",
  baseUrl: "mathstruct.org/Sophia",
  ignorePatterns: [
    // the site generator itself lives inside the vault
    "site",
    // repository documentation; the site has its own landing page (index.md)
    "README.md",
    "AGENTS.md",
    "CONTRIBUTING.md",
    // working material (the prompts used while drafting the vault), not part of the design
    "meta",
    // sources and manifests: every .rs/.jl has a .md sibling that *is* published
    "**/*.rs",
    "**/*.jl",
    "**/*.toml",
    "Cargo.lock",
    "target",
    "templates",
  ],
  // Formulas are written in Typst (the vault uses wypst). Each one is offered to Typst first;
  // anything Typst rejects falls through to KaTeX, so a stray LaTeX formula still renders.
  math: ["typst", "katex"],
})
