# rs-markdown

Read Markdown files in a scrollable terminal viewer:

```sh
cargo run -- README.md
```

The viewer supports headings, inline formatting, links, nested and task lists,
blockquotes, code blocks, tables, and horizontal rules. Use the arrow keys or
`j`/`k` to scroll, `PageUp`/`PageDown` to move by a page, `Home`/`End` to jump,
`/` to search, `n`/`N` to navigate matches, and `q` or `Esc` to quit.

Markdown images are previewed inline using colored terminal half-blocks. Local
image paths are resolved relative to the Markdown file; HTTP(S) URLs are
downloaded with size and timeout limits. PNG, JPEG, GIF, WebP, BMP, and SVG are
supported.

Render the document to stdout without opening the interactive viewer:

```sh
cargo run -- --static README.md
```