import { QuartzTransformerPlugin } from "../types"
import { Root, Code, RootContent } from "mdast"
import { visit } from "unist-util-visit"

/**
 * Renders ````tabs blocks of the Obsidian "Markdown Tabs" plugin:
 *
 *   ````tabs
 *   tab: Julia
 *   ```julia
 *   ...
 *   ```
 *   tab: Haskell
 *   ...
 *   ````
 *
 * Each tab body may contain fenced code blocks (kept as `code` nodes so the
 * normal syntax highlighter handles them) and plain text lines.
 */
type Tab = { title: string; nodes: RootContent[] }

function parseTabs(src: string): Tab[] {
  const tabs: Tab[] = []
  let cur: Tab | null = null
  let fence: { lang: string; marker: string; lines: string[] } | null = null
  let text: string[] = []
  const flushText = () => {
    const t = text.join("\n").trim()
    if (t && cur) cur.nodes.push({ type: "paragraph", children: [{ type: "text", value: t }] })
    text = []
  }
  for (const line of src.split("\n")) {
    if (fence) {
      if (line.trim() === fence.marker) {
        cur!.nodes.push({
          type: "code",
          lang: fence.lang || null,
          value: fence.lines.join("\n"),
        } as Code)
        fence = null
      } else fence.lines.push(line)
      continue
    }
    const m = line.match(/^tab:\s*(.*)$/)
    if (m) {
      flushText()
      cur = { title: m[1].trim(), nodes: [] }
      tabs.push(cur)
      continue
    }
    const f = line.match(/^(`{3,})(\S*)\s*$/)
    if (f && cur) {
      flushText()
      fence = { lang: f[2], marker: f[1], lines: [] }
      continue
    }
    if (cur) text.push(line)
  }
  flushText()
  return tabs
}

const escape = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;")

export const ObsidianTabs: QuartzTransformerPlugin = () => ({
  name: "ObsidianTabs",
  markdownPlugins() {
    return [
      () => (tree: Root) => {
        visit(tree, "code", (node: Code, index, parent) => {
          if (node.lang !== "tabs" || !parent || index === undefined) return
          const tabs = parseTabs(node.value)
          if (tabs.length === 0) return
          const nav = tabs
            .map(
              (t, i) =>
                `<button type="button" class="tab-button${i === 0 ? " active" : ""}" data-tab="${i}">${escape(t.title)}</button>`,
            )
            .join("")
          const out: RootContent[] = [
            {
              type: "html",
              value: `<div class="obsidian-tabs"><div class="tab-nav" role="tablist">${nav}</div>`,
            } as any,
          ]
          tabs.forEach((t, i) => {
            out.push({
              type: "html",
              value: `<div class="tab-panel${i === 0 ? " active" : ""}" data-tab="${i}">`,
            } as any)
            out.push(...t.nodes)
            out.push({ type: "html", value: `</div>` } as any)
          })
          out.push({ type: "html", value: `</div>` } as any)
          parent.children.splice(index, 1, ...out)
          return index + out.length
        })
      },
    ]
  },
  externalResources() {
    return {
      js: [
        {
          loadTime: "afterDOMReady",
          contentType: "inline",
          script: `document.addEventListener("click", (e) => {
  const btn = e.target.closest(".obsidian-tabs .tab-button"); if (!btn) return;
  const root = btn.closest(".obsidian-tabs"); const id = btn.dataset.tab;
  root.querySelectorAll(":scope > .tab-nav > .tab-button").forEach(b => b.classList.toggle("active", b.dataset.tab === id));
  root.querySelectorAll(":scope > .tab-panel").forEach(p => p.classList.toggle("active", p.dataset.tab === id));
});`,
        },
      ],
    }
  },
})
