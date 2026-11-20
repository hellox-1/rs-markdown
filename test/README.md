# RMD — Markdown Rendering Test Document

> A comprehensive Markdown test document for developing and testing `rmd`, a lightweight terminal Markdown viewer written in Rust.

**Version:** 0.1.0  
**Author:** RMD Development Team  
**Status:** Development

---

# 1. Introduction

Welcome to the RMD Markdown rendering test document.

The purpose of this document is to test how well our terminal application handles different Markdown elements, formatting styles, nested structures, code blocks, tables, and other content.

RMD aims to provide a beautiful reading experience directly inside the terminal without requiring a browser or graphical interface.

## 1.1 Project Goals

- Provide clean Markdown rendering.
- Support interactive terminal navigation.
- Offer static rendering mode.
- Highlight source code.
- Handle large Markdown documents efficiently.
- Support different terminal dimensions.

## 1.2 Important Notice

This document is intentionally long and contains multiple formatting styles to test the rendering engine.

---

# 2. Text Formatting

This section tests different inline Markdown formatting features.

## 2.1 Basic Formatting

Normal text should appear without any special styling.

**This text is bold.**

*This text is italic.*

***This text is bold and italic.***

~~This text is crossed out.~~

You can also combine formatting:

**This sentence contains *bold and italic* formatting together.**

Here is some `inline code` inside a normal paragraph.

## 2.2 Long Paragraph

Markdown viewers must handle long paragraphs correctly. The following paragraph is intentionally lengthy to test automatic line wrapping, terminal width detection, text alignment, and scrolling behavior. When a paragraph exceeds the available terminal width, the rendering engine should automatically wrap the content without cutting words unnecessarily. It should also preserve the spacing between paragraphs and ensure that formatting remains consistent even when a line wraps across multiple terminal rows. This becomes particularly important when users open documentation files, technical guides, README files, or long-form notes inside a terminal window.

## 2.3 Links

- [Rust Official Website](https://www.rust-lang.org/)
- [Ratatui Documentation](https://ratatui.rs/)
- [GitHub](https://github.com/)
- [Crates.io](https://crates.io/)

---

# 3. Headings

The following headings test all six Markdown heading levels.

# Heading Level 1

## Heading Level 2

### Heading Level 3

#### Heading Level 4

##### Heading Level 5

###### Heading Level 6

Headings should have different visual hierarchy through size-like emphasis, color, weight, and spacing.

---

# 4. Lists

## 4.1 Unordered Lists

- Rust
- JavaScript
- TypeScript
- Python
- C
- C++

## 4.2 Nested Unordered Lists

- Programming Languages
  - Rust
  - Python
  - C++
- Frameworks
  - React
  - Next.js
  - Axum
- Databases
  - PostgreSQL
  - MongoDB
  - SQLite

## 4.3 Ordered Lists

1. Create a new Rust project.
2. Configure dependencies.
3. Design the document model.
4. Implement Markdown parsing.
5. Build the rendering engine.
6. Implement terminal navigation.
7. Test the application.
8. Release the project.

## 4.4 Nested Ordered Lists

1. Frontend
   1. Components
   2. Styling
   3. State management
2. Backend
   1. API design
   2. Authentication
   3. Database
3. Deployment
   1. Build
   2. Docker
   3. Production

## 4.5 Task Lists

- [x] Initialize Cargo project
- [x] Select core dependencies
- [ ] Implement Markdown parser
- [ ] Create document representation
- [ ] Build static renderer
- [ ] Build interactive viewer
- [ ] Add syntax highlighting
- [ ] Publish to crates.io

---

# 5. Code Blocks

## 5.1 Rust

```rust
use std::fs;
use std::io;

fn read_markdown(path: &str) -> io::Result<String> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

fn main() {
    match read_markdown("README.md") {
        Ok(content) => {
            println!("{}", content);
        }
        Err(error) => {
            eprintln!("Failed to read file: {}", error);
        }
    }
}
```

## 5.2 JavaScript

```javascript
function greet(name) {
    return `Hello, ${name}!`;
}

const users = ["Alice", "Bob", "Charlie"];

users.forEach((user) => {
    console.log(greet(user));
});
```

## 5.3 TypeScript

```typescript
interface User {
    id: number;
    name: string;
    email: string;
}

function getUser(id: number): User | null {
    if (id <= 0) {
        return null;
    }

    return {
        id,
        name: "Sanchit",
        email: "user@example.com"
    };
}
```

## 5.4 Python

```python
class MarkdownDocument:
    def __init__(self, title):
        self.title = title
        self.content = []

    def add_section(self, section):
        self.content.append(section)

    def display(self):
        print(f"# {self.title}")

document = MarkdownDocument("RMD")
document.add_section("Introduction")
document.display()
```

## 5.5 C

```c
#include <stdio.h>

int main(void) {
    int numbers[] = {10, 20, 30, 40, 50};

    for (int i = 0; i < 5; i++) {
        printf("%d\n", numbers[i]);
    }

    return 0;
}
```

## 5.6 Bash

```bash
#!/usr/bin/env bash

echo "Building RMD..."

cargo build --release

if [ $? -eq 0 ]; then
    echo "Build successful!"
else
    echo "Build failed!"
fi
```

## 5.7 JSON

```json
{
    "name": "rmd",
    "version": "0.1.0",
    "description": "A terminal markdown viewer",
    "features": [
        "markdown-rendering",
        "interactive-viewer",
        "syntax-highlighting"
    ],
    "active": true
}
```

---

# 6. Tables

## 6.1 Programming Languages

| Language | Type | Primary Use |
|---|---|---|
| Rust | Systems | CLI, Backend |
| JavaScript | Scripting | Web Development |
| TypeScript | Typed Scripting | Full Stack |
| Python | General Purpose | AI, Automation |
| C | Systems | Operating Systems |
| C++ | Systems | Game Development |

## 6.2 Alignment Test

| Left Aligned | Center Aligned | Right Aligned |
|:---|:---:|---:|
| Apple | 100 | $10 |
| Banana | 200 | $20 |
| Orange | 300 | $30 |
| Mango | 400 | $40 |

## 6.3 Large Table

| ID | Name | Department | Role | Experience | Status |
|---:|---|---|---|---:|---|
| 1 | Alice | Engineering | Developer | 3 years | Active |
| 2 | Bob | Design | Designer | 2 years | Active |
| 3 | Charlie | Engineering | Architect | 8 years | Active |
| 4 | David | Marketing | Manager | 5 years | Inactive |
| 5 | Emma | Security | Engineer | 4 years | Active |
| 6 | Frank | Engineering | Tester | 2 years | Active |
| 7 | Grace | Research | Scientist | 6 years | Active |
| 8 | Henry | Operations | Administrator | 7 years | Active |

---

# 7. Blockquotes

> This is a simple blockquote.
>
> It contains multiple lines of text.

> **Important**
>
> Always validate input before processing user-provided files.

Nested blockquotes:

> This is the outer blockquote.
>
> > This is the inner blockquote.
> >
> > Nested content should be visually distinguishable.

---

# 8. Horizontal Rules

First section.

---

Second section.

***

Third section.

___

Fourth section.

---

# 9. Mathematical and Technical Content

Einstein's famous equation:

$$E = mc^2$$

Quadratic formula:

$$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$$

Inline mathematical expression: \(a^2 + b^2 = c^2\).

---

# 10. Special Characters and Unicode

Testing special characters is important for terminal applications.

| Character | Description |
|---|---|
| → | Right arrow |
| ← | Left arrow |
| ↑ | Up arrow |
| ↓ | Down arrow |
| ✓ | Check mark |
| ✗ | Cross mark |
| ★ | Star |
| ♥ | Heart |
| λ | Lambda |
| π | Pi |
| ∑ | Summation |
| ∞ | Infinity |

## 10.1 Different Languages

- English: Hello World
- Hindi: नमस्ते दुनिया
- Bengali: নমস্কার পৃথিবী
- Japanese: こんにちは世界
- Chinese: 你好世界
- Russian: Привет мир

## 10.2 Emoji

🚀 Development

🔥 Performance

💻 Programming

🦀 Rust

📚 Documentation

⚡ Speed

---

# 11. Nested Content

> ## Important Development Notes
>
> RMD should support nested Markdown structures.
>
> - Parsing
> - Rendering
>   - Static rendering
>   - Interactive rendering
> - Testing
>
> ```rust
> fn main() {
>     println!("Nested code block");
> }
> ```

---

# 12. Large Content Test

## 12.1 Performance

A terminal Markdown viewer should remain responsive even when the input document contains thousands of lines. Rendering should avoid unnecessary work, especially in interactive mode where only a portion of the document is visible at any given time.

The parser should process the Markdown document and produce a reusable representation. The renderer should then consume that representation rather than repeatedly parsing the original source every time the user scrolls.

## 12.2 Memory Management

Memory consumption matters when processing large documentation files. The application should avoid unnecessary string cloning, excessive allocations, and repeated transformations.

Rust provides ownership and borrowing mechanisms that can help us maintain predictable memory usage while keeping the implementation safe.

## 12.3 Terminal Compatibility

Different terminal emulators have different capabilities. Some support advanced colors, Unicode symbols, hyperlinks, and image protocols, while others offer only basic text rendering.

RMD should provide a sensible fallback when advanced terminal features are unavailable.

## 12.4 Navigation

Interactive mode should support:

- Arrow keys for line-by-line navigation.
- Page Up and Page Down for larger jumps.
- Home to jump to the beginning.
- End to jump to the document's end.
- `/` to search.
- `n` to find the next match.
- `q` to quit.

## 12.5 Future Improvements

Potential future improvements include:

1. Custom themes.
2. Configuration files.
3. Search highlighting.
4. Image previews.
5. Hyperlink support.
6. Live file reloading.
7. Mouse scrolling.
8. Terminal resize handling.
9. Better table layouts.
10. Large-file optimization.

---

# 13. Final Testing Checklist

Use this checklist while developing the RMD rendering engine.

- [ ] Heading hierarchy
- [ ] Bold and italic formatting
- [ ] Strikethrough
- [ ] Inline code
- [ ] Code blocks
- [ ] Nested lists
- [ ] Ordered lists
- [ ] Task lists
- [ ] Tables
- [ ] Table alignment
- [ ] Blockquotes
- [ ] Nested blockquotes
- [ ] Horizontal rules
- [ ] Links
- [ ] Unicode characters
- [ ] Emoji rendering
- [ ] Long paragraphs
- [ ] Long code lines
- [ ] Empty document
- [ ] Very large document

---

# 14. Conclusion

RMD is intended to make reading Markdown files in the terminal a better experience.

The project prioritizes simplicity, performance, readability, and a clean user interface.

**Read Markdown. Beautifully. Inside your terminal.**

`rmd v0.1.0` — Built with Rust 🦀

---
# 15. Image Rendering

## 15.1 Remote Image

![Rust Ferris](https://www.rust-lang.org/static/images/rust-logo-blk.svg)

## 15.2 Landscape Image

![Mountain Landscape](https://images.unsplash.com/photo-1500530855697-b586d89ba3ee?w=1200)

## 15.3 Image with Description

![Nature](https://images.unsplash.com/photo-1470770841072-f978cf4d019e?w=1200)

*Figure: A beautiful landscape.*

## 15.4 Multiple Images

![Forest](https://images.unsplash.com/photo-1448375240586-882707db888b?w=800)

![Ocean](https://images.unsplash.com/photo-1507525428034-b723cf961d3e?w=800)
*End of Markdown rendering test document.*
