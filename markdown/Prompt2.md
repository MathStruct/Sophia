Please go through this obsidian vault again and extend it.

You may write the formulas either in Latex or Typst. Can can create TikZ diagrams.
```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}
X \arrow[r, "f"] \arrow[dr, "f"'] \arrow[d, "\mathrm{id}_X"'] & Y \arrow[d, "\mathrm{id}_Y"] \\
X \arrow[r, "f"'] & Y
\end{tikzcd}
\end{document}
```
An addon will automatically render the code.


Please do an extensive survey on the idea and insert every detail you find in the obsidian. 

Plan a layout for the repository and create .jl and .rs file but DO NOT RUN THE CODE (keep the code conceptual and incomplete). only write in comments what you intend this file to do.  Document this idea in a markdown file right next to the .jl or .rs file under the same name. Insert these files into the markdown vault.