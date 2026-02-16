use anyhow::Result;
use std::fs;
use std::path::Path;

pub mod config;

#[derive(Debug, PartialEq)]
pub enum FileResult {
    /// File was unchanged (already clean)
    Unchanged,
    /// File was modified (or would be modified in check mode)
    Changed,
}

/// Determine if a line starts an import statement.
pub fn is_import_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with("import ") || trimmed.starts_with("import(") {
        // Exclude `import.meta.glob` — handled separately
        if trimmed.starts_with("import.meta") {
            return false;
        }
        return true;
    }
    if trimmed == "import" {
        return true;
    }
    false
}

/// Determine if a line is part of an import.meta expression (single or multiline).
fn is_import_meta_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("import.meta")
}

/// Track whether we are inside a multiline construct (import or import.meta).
/// Returns the new `in_multiline` state.
pub fn is_in_multiline_import(line: &str, in_multiline: bool) -> bool {
    if in_multiline {
        // We're inside a multiline import/expression.
        // Check if this line closes it.
        let trimmed = line.trim();
        // Count braces/parens to detect closure (simple heuristic)
        if trimmed.contains('}') || trimmed.ends_with(')') || trimmed.ends_with(");") {
            return false;
        }
        return true;
    }

    // Not currently in multiline — check if this line opens one
    let trimmed = line.trim();

    // Multiline import: has `{` but no `}` on same line
    if is_import_line(trimmed) && trimmed.contains('{') && !trimmed.contains('}') {
        return true;
    }

    // import.meta.glob(...) multiline: has `(` but no `)` on same line
    if is_import_meta_line(trimmed) && trimmed.contains('(') && !trimmed.contains(')') {
        return true;
    }

    false
}

/// Returns true if line is a comment (single-line or block comment start/end).
fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") || trimmed.ends_with("*/")
}

/// Core transform: remove blank lines between import statements.
/// Pure function — no I/O.
pub fn squeeze_imports(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<&str> = Vec::with_capacity(lines.len());
    let mut in_multiline = false;
    let mut in_import_block = false;
    let mut pending_blank_lines: Vec<&str> = Vec::new();
    let mut pending_comment_lines: Vec<&str> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        let is_blank = trimmed.is_empty();
        let is_comment = is_comment_line(trimmed);
        let is_import = is_import_line(trimmed) || is_import_meta_line(trimmed) || in_multiline;

        if in_multiline {
            // Continue multiline import — always include
            result.push(line);
            in_multiline = is_in_multiline_import(line, true);
            continue;
        }

        if is_import {
            in_import_block = true;
            // We hit an import line — discard any pending blank lines
            // but keep comment lines that were between imports
            // Actually: discard blank lines between imports, keep comments
            // Re-think: we discard blank lines between imports and also between
            // comments that are sandwiched between imports.
            // Flush pending comments (they are between imports)
            for cl in pending_comment_lines.drain(..) {
                result.push(cl);
            }
            pending_blank_lines.clear();
            result.push(line);
            in_multiline = is_in_multiline_import(line, false);
            continue;
        }

        if in_import_block {
            if is_blank {
                pending_blank_lines.push(line);
                continue;
            }
            if is_comment {
                pending_comment_lines.push(line);
                continue;
            }
            // Non-import, non-blank, non-comment line while we were in import block
            // => import block ended. Flush pending blanks and comments.
            in_import_block = false;
            for bl in pending_blank_lines.drain(..) {
                result.push(bl);
            }
            for cl in pending_comment_lines.drain(..) {
                result.push(cl);
            }
            result.push(line);
            continue;
        }

        // Not in import block — pass through
        result.push(line);
    }

    // Flush any remaining pending lines
    for bl in pending_blank_lines.drain(..) {
        result.push(bl);
    }
    for cl in pending_comment_lines.drain(..) {
        result.push(cl);
    }

    let mut output = result.join("\n");
    // Preserve trailing newline if original had one
    if content.ends_with('\n') {
        output.push('\n');
    }
    output
}

/// Process a single file. Returns whether the file was changed.
/// In check mode, does not write to disk.
pub fn process_file(path: &Path, check: bool) -> Result<FileResult> {
    let content = fs::read_to_string(path)?;
    let squeezed = squeeze_imports(&content);

    if squeezed == content {
        return Ok(FileResult::Unchanged);
    }

    if !check {
        fs::write(path, &squeezed)?;
    }

    Ok(FileResult::Changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_import_line() {
        assert!(is_import_line("import { useState } from 'react'"));
        assert!(is_import_line("import type { FC } from 'react'"));
        assert!(is_import_line("import './styles.css'"));
        assert!(is_import_line("  import { foo } from 'bar'"));
        assert!(!is_import_line("const x = 1"));
        assert!(!is_import_line("// import something"));
        assert!(!is_import_line("import.meta.glob('./**/*.ts')"));
        assert!(!is_import_line(""));
    }

    #[test]
    fn test_basic_squeeze() {
        let input = "\
import { useState } from 'react'

import { Button } from '@/components'

import { api } from '@/lib/api'

const foo = 'bar'
";
        let expected = "\
import { useState } from 'react'
import { Button } from '@/components'
import { api } from '@/lib/api'

const foo = 'bar'
";
        assert_eq!(squeeze_imports(input), expected);
    }

    #[test]
    fn test_multiline_import() {
        let input = "\
import {
  useState,
  useEffect,
} from 'react'

import { Button } from '@/components'

const foo = 'bar'
";
        let expected = "\
import {
  useState,
  useEffect,
} from 'react'
import { Button } from '@/components'

const foo = 'bar'
";
        assert_eq!(squeeze_imports(input), expected);
    }

    #[test]
    fn test_no_imports() {
        let input = "const x = 1\nconst y = 2\n";
        assert_eq!(squeeze_imports(input), input);
    }

    #[test]
    fn test_already_clean() {
        let input = "\
import { a } from 'a'
import { b } from 'b'

const x = 1
";
        assert_eq!(squeeze_imports(input), input);
    }

    #[test]
    fn test_import_type() {
        let input = "\
import type { FC } from 'react'

import { useState } from 'react'

export const App: FC = () => {}
";
        let expected = "\
import type { FC } from 'react'
import { useState } from 'react'

export const App: FC = () => {}
";
        assert_eq!(squeeze_imports(input), expected);
    }

    #[test]
    fn test_import_meta_glob_single_line() {
        let input = "\
import { api } from '@/lib'

import.meta.glob('./**/*.ts')

const x = 1
";
        let expected = "\
import { api } from '@/lib'
import.meta.glob('./**/*.ts')

const x = 1
";
        assert_eq!(squeeze_imports(input), expected);
    }

    #[test]
    fn test_import_meta_glob_multiline() {
        let input = "\
import { api } from '@/lib'

import.meta.glob(
  './**/*.ts',
  { eager: true }
)

const x = 1
";
        let expected = "\
import { api } from '@/lib'
import.meta.glob(
  './**/*.ts',
  { eager: true }
)

const x = 1
";
        assert_eq!(squeeze_imports(input), expected);
    }

    #[test]
    fn test_top_comment_then_imports() {
        let input = "\
// @ts-nocheck

import { a } from 'a'

import { b } from 'b'

const x = 1
";
        let expected = "\
// @ts-nocheck

import { a } from 'a'
import { b } from 'b'

const x = 1
";
        assert_eq!(squeeze_imports(input), expected);
    }

    #[test]
    fn test_side_effect_import() {
        let input = "\
import './polyfill'

import { useState } from 'react'

const x = 1
";
        let expected = "\
import './polyfill'
import { useState } from 'react'

const x = 1
";
        assert_eq!(squeeze_imports(input), expected);
    }

    #[test]
    fn test_multiple_blank_lines_between_imports() {
        let input = "\
import { a } from 'a'



import { b } from 'b'

const x = 1
";
        let expected = "\
import { a } from 'a'
import { b } from 'b'

const x = 1
";
        assert_eq!(squeeze_imports(input), expected);
    }

    #[test]
    fn test_preserves_blank_lines_after_imports() {
        let input = "\
import { a } from 'a'
import { b } from 'b'


const x = 1


const y = 2
";
        assert_eq!(squeeze_imports(input), input);
    }

    #[test]
    fn test_no_trailing_newline() {
        let input = "import { a } from 'a'\n\nimport { b } from 'b'";
        let expected = "import { a } from 'a'\nimport { b } from 'b'";
        assert_eq!(squeeze_imports(input), expected);
    }
}
