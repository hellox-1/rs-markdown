# rs-markdown

Read Markdown files in a scrollable terminal viewer with `rmd`:

```sh
cargo run -- README.md
```

## Install

Download the archive for your operating system from [GitHub Releases](https://github.com/hellox-1/rs-markdown/releases),
extract it, and put the `rmd` executable (or `rmd.exe` on
Windows) somewhere on your `PATH`. Or install from a local checkout with:

```sh
cargo install --path .
```

Release builds are produced for 64-bit Linux, Windows, and macOS. To build for
another Rust-supported target, install its Rust target and run
`cargo build --release --target <target-triple>` on a compatible toolchain.

The viewer supports headings, inline formatting, links, nested and task lists,
blockquotes, code blocks, tables, and horizontal rules. Use the arrow keys or
`j`/`k` to scroll, `PageUp`/`PageDown` to move by a page, `Home`/`End` to jump,
`/` to search, `n`/`N` to navigate matches, and `q` or `Esc` to quit.

## Azure DevOps Markdown

The renderer follows the [Azure DevOps Markdown guidance](https://learn.microsoft.com/en-us/azure/devops/project/wiki/markdown-guidance?view=azure-devops),
including CommonMark/GFM formatting, escaped Markdown, aligned tables, task
lists, emoji shortcodes, HTML text formatting, wiki table-of-contents tags,
mentions, image dimensions, and Mermaid diagrams rendered as terminal text.

Markdown images are previewed inline using colored terminal half-blocks. Local
image paths are resolved relative to the Markdown file; HTTP(S) URLs are
downloaded with size and timeout limits. PNG, JPEG, GIF, WebP, BMP, and SVG are
supported.

Azure DevOps features that require a live wiki or project aren't available
offline: subpage and query-result macros show explanatory text, video embeds
show their URL, and task checkboxes, mentions, and links aren't interactive.
Math notation is shown as readable source rather than typeset KaTeX, and
unsupported HTML or Mermaid constructs fall back to text.

Render the document to stdout without opening the interactive viewer:

```sh
cargo run -- --static README.md
```