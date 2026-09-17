import remarkMath from "remark-math"
import rehypeKatex from "rehype-katex"
import rehypeMathjax from "rehype-mathjax/svg"
//@ts-ignore
import rehypeTypst from "@myriaddreamin/rehype-typst"
import { SKIP, visit } from "unist-util-visit"
import { Element, ElementContent, Root } from "hast"
import { Code, Root as MdRoot } from "mdast"
import { VFile } from "vfile"
import { QuartzTransformerPlugin } from "../types"
import { KatexOptions } from "katex"
import { Options as MathjaxOptions } from "rehype-mathjax/svg"
//@ts-ignore
import { Options as TypstOptions } from "@myriaddreamin/rehype-typst"

export type RenderEngine = "katex" | "mathjax" | "typst"

interface Options {
  /**
   * Engines to try, in order. With more than one, each formula is rendered by
   * the first engine that can handle it — which is what lets a single note mix
   * Typst and LaTeX.
   */
  renderEngine: RenderEngine | RenderEngine[]
  customMacros: MacroType
  katexOptions: Omit<KatexOptions, "macros" | "output">
  mathJaxOptions: Omit<MathjaxOptions, "macros">
  typstOptions: TypstOptions
  /** Log a line for each note that ends up using more than one engine. */
  warnOnMixed: boolean
}

// mathjax macros
export type Args = boolean | number | string | null
interface MacroType {
  [key: string]: string | Args[]
}

type Transformer = (tree: Root, file: VFile) => void | Promise<void>

// what an author may write in `math:` frontmatter
const engineAliases: Record<string, RenderEngine> = {
  katex: "katex",
  latex: "katex",
  tex: "katex",
  mathjax: "mathjax",
  typst: "typst",
  wypst: "typst",
}

/**
 * Which engines this page may use. A `math:` key pins the page to one engine —
 * useful to keep a note honest once it has been cleaned up — while anything
 * else falls back to the configured chain.
 */
function chainFor(file: VFile, configured: RenderEngine[]): RenderEngine[] {
  const frontmatter = file.data.frontmatter as Record<string, unknown> | undefined
  const declared = frontmatter?.math ?? frontmatter?.mathEngine
  if (typeof declared !== "string") return configured
  const key = declared.trim().toLowerCase()
  if (key === "auto" || key === "mixed") return configured
  const pinned = engineAliases[key]
  return pinned ? [pinned] : configured
}

/**
 * Typst math never spells an operator as a backslash command, LaTeX almost
 * always does, so a backslash followed by a letter is a reliable tell. Catching
 * these up front matters because Typst *accepts* a few of them and renders
 * nonsense (`\hat{y} = X\beta`, `e^{i\pi}`) rather than failing.
 */
function looksLikeLatex(src: string): boolean {
  return /\\[a-zA-Z]/.test(src)
}

// marks a ```typst fence, which renders with Typst whatever the page engine is
const typstFenceClass = "typst-fence"

function classesOf(node: Element): string[] {
  const className = node.properties?.className
  return Array.isArray(className) ? (className as string[]) : []
}

/**
 * Turn a ```typst fence into display math before the syntax highlighter can
 * claim it. The shape mirrors what mdast-util-math emits for `$$…$$`, plus a
 * marker class so it can be held to Typst rather than falling back.
 */
function typstFencesToMath() {
  return (tree: MdRoot) => {
    visit(tree, "code", (node: Code, index, parent) => {
      if (node.lang !== "typst" || !parent || index === undefined) return
      parent.children[index] = {
        type: "math",
        value: node.value,
        data: {
          hName: "pre",
          hChildren: [
            {
              type: "element",
              tagName: "code",
              properties: {
                className: ["language-math", "math-display", typstFenceClass],
              },
              children: [{ type: "text", value: node.value }],
            },
          ],
        },
      } as any
    })
  }
}

interface MathTarget {
  /** node to replace once the formula is rendered */
  scope: Element
  parent: Root | Element
  code: Element
  display: boolean
  /** came from a ```typst fence, so it is Typst by construction */
  fence: boolean
}

function collectMath(tree: Root): MathTarget[] {
  const targets: MathTarget[] = []
  visit(tree, "element", (node: Element, _index, parent) => {
    if (!parent || (parent.type !== "root" && parent.type !== "element")) return

    // display math: <pre><code class="language-math math-display">
    if (node.tagName === "pre") {
      const code = node.children.find(
        (child): child is Element => child.type === "element" && child.tagName === "code",
      )
      if (!code || !classesOf(code).includes("language-math")) return
      targets.push({
        scope: node,
        parent,
        code,
        display: true,
        fence: classesOf(code).includes(typstFenceClass),
      })
      return SKIP
    }

    // inline math: a bare <code class="language-math math-inline">
    if (node.tagName === "code" && classesOf(node).includes("language-math")) {
      targets.push({
        scope: node,
        parent,
        code: node,
        display: classesOf(node).includes("math-display"),
        fence: classesOf(node).includes(typstFenceClass),
      })
    }
  })
  return targets
}

function sourceOf(code: Element): string {
  let source = ""
  visit(code, "text", (node) => {
    source += node.value
  })
  return source
}

function replaceScope(target: MathTarget, replacement: ElementContent[]) {
  const index = target.parent.children.indexOf(target.scope)
  if (index === -1) return
  target.parent.children.splice(index, 1, ...replacement)
}

/**
 * Render one formula with Typst, in isolation so a failure cannot take the rest
 * of the page (or the build) with it. rehype-typst signals a failed compile by
 * either throwing or leaving a `.typst-error` behind; both mean "not Typst".
 */
async function renderWithTypst(
  target: MathTarget,
  transform: Transformer,
): Promise<ElementContent[] | null> {
  const isolated: Root = { type: "root", children: [target.scope] }
  // a rejected compile is an expected, routine outcome here -- it is how a
  // LaTeX formula is identified -- but the Typst compiler dumps its diagnostics
  // to the console regardless, so mute it for the duration of the attempt
  const consoleError = console.error
  console.error = () => {}
  try {
    await transform(isolated, new VFile({ path: "math" }))
  } catch {
    return null
  } finally {
    console.error = consoleError
  }
  const rendered = isolated.children as ElementContent[]
  if (rendered.length === 1 && rendered[0] === target.scope) return null
  if (rendered.some((node) => node.type === "element" && classesOf(node).includes("typst-error"))) {
    return null
  }
  return rendered
}

function mathError(target: MathTarget, source: string): ElementContent {
  return {
    type: "element",
    tagName: target.display ? "pre" : "span",
    properties: { className: ["math-error"], title: "Typst could not render this formula" },
    children: [{ type: "text", value: source }],
  }
}

/**
 * rehype-typst hard-codes layout into a `style` attribute (and emits a broken
 * `vertical-align: -NaNem` for display math). Swap that for a class so the
 * stylesheet owns how a formula sits in the page.
 */
function tidyTypstOutput(tree: Root) {
  visit(tree, "element", (node: Element) => {
    if (node.tagName !== "svg" || !classesOf(node).includes("typst-doc")) return
    const style = String(node.properties?.style ?? "").replace(/\s/g, "")
    if (!style.includes("display:block")) return
    node.properties!.className = [...classesOf(node), "typst-display"]
    delete node.properties!.style
  })
}

export const Latex: QuartzTransformerPlugin<Partial<Options>> = (opts) => {
  const configured = opts?.renderEngine ?? "katex"
  const defaultChain: RenderEngine[] = Array.isArray(configured) ? configured : [configured]
  const macros = opts?.customMacros ?? {}
  const warnOnMixed = opts?.warnOnMixed ?? true

  // renderers are built on first use: the Typst compiler is expensive to spin
  // up and most pages never need it
  const renderers: Partial<Record<RenderEngine, Transformer>> = {}
  const rendererFor = (engine: RenderEngine): Transformer =>
    (renderers[engine] ??= {
      katex: () =>
        rehypeKatex({ output: "html", macros, ...(opts?.katexOptions ?? {}) }) as Transformer,
      typst: () => rehypeTypst(opts?.typstOptions ?? {}) as Transformer,
      mathjax: () =>
        rehypeMathjax({
          ...(opts?.mathJaxOptions ?? {}),
          // mathjax accepts `macros` at runtime; its published types do not
          tex: { ...(opts?.mathJaxOptions?.tex ?? {}), macros } as MathjaxOptions["tex"],
        }) as Transformer,
    }[engine]())

  return {
    name: "Latex",
    markdownPlugins() {
      return [remarkMath, typstFencesToMath]
    },
    htmlPlugins() {
      return [
        () => async (tree: Root, file: VFile) => {
          const chain = chainFor(file, defaultChain)
          const targets = collectMath(tree)
          if (targets.length === 0) return

          // Typst is tried per formula because it is the only engine that fails
          // loudly on the other's syntax. KaTeX and MathJax render anything
          // without complaint, so whichever of them is in the chain goes last
          // and mops up everything Typst did not take.
          const lastResort = chain.find((engine) => engine !== "typst")
          let typstCount = 0
          let lastResortCount = 0

          if (chain.includes("typst")) {
            const transform = rendererFor("typst")
            for (const target of targets) {
              const source = sourceOf(target.code)

              if (!target.fence && lastResort && looksLikeLatex(source)) {
                lastResortCount++
                continue
              }

              const rendered = await renderWithTypst(target, transform)
              if (rendered) {
                replaceScope(target, rendered)
                typstCount++
              } else if (target.fence || !lastResort) {
                // an explicit Typst fence, or a page pinned to Typst: say so
                // rather than quietly rendering it as something else
                replaceScope(target, [mathError(target, source)])
              } else {
                lastResortCount++
              }
            }
            tidyTypstOutput(tree)
          }

          if (lastResort) await rendererFor(lastResort)(tree, file)

          if (warnOnMixed && typstCount > 0 && lastResortCount > 0) {
            console.warn(
              `\nMixed math in ${file.path}: ${typstCount} formula(s) rendered with Typst, ` +
                `${lastResortCount} with ${lastResort}`,
            )
          }
        },
      ]
    },
    externalResources() {
      // pages opt in and out of engines individually, so KaTeX's stylesheet is
      // always shipped rather than keyed off the configured engine
      return {
        css: [{ content: "https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" }],
        js: [
          {
            // fix copy behaviour: https://github.com/KaTeX/KaTeX/blob/main/contrib/copy-tex/README.md
            src: "https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/copy-tex.min.js",
            loadTime: "afterDOMReady",
            contentType: "external",
          },
        ],
      }
    },
  }
}
