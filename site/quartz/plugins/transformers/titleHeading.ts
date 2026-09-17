import { Root } from "mdast"
import { toString } from "mdast-util-to-string"
import { QuartzTransformerPlugin } from "../types"

/**
 * Obsidian notes conventionally open with `# <note name>`, and Quartz already
 * renders the note name as the article title, so the title would appear twice
 * on every page. Drop a leading level-1 heading when its text is the page
 * title. A heading that says something else is left alone.
 *
 * Must run after FrontMatter (which sets the title) and before TableOfContents
 * (so the heading does not appear in the outline).
 */
export const DropTitleHeading: QuartzTransformerPlugin = () => ({
  name: "DropTitleHeading",
  markdownPlugins() {
    return [
      () => (tree: Root, file) => {
        const first = tree.children[0]
        if (!first || first.type !== "heading" || first.depth !== 1) return
        const title = file.data.frontmatter?.title
        if (typeof title !== "string") return
        if (toString(first).trim().toLowerCase() === title.trim().toLowerCase()) {
          tree.children.shift()
        }
      },
    ]
  },
})
