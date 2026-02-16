# import-squeeze

Fast Rust CLI that squeezes blank lines between import statements, so Biome sorts them as one group.

## Why?

Biome treats line-break-separated import groups independently when sorting. To get a single sorted block you need to remove those blank lines first. This tool does exactly that — and nothing else — replacing the slow ESLint plugin that was the bottleneck.

## Before / After

```ts
// before
import { useState } from 'react'

import { Button } from '@/components'

import { api } from '@/lib/api'

const foo = 'bar'

// after
import { useState } from 'react'
import { Button } from '@/components'
import { api } from '@/lib/api'

const foo = 'bar'
```

## Install

```bash
npm install -D import-squeeze
```

No Rust toolchain required — prebuilt binaries for macOS (arm64/x64), Linux (x64), Windows (x64).

## Usage

```bash
# Process specific files (lint-staged friendly)
import-squeeze src/App.tsx src/main.ts

# Process all files from biome.json includes
import-squeeze

# CI: check without modifying (exit code 1 if changes needed)
import-squeeze --check

# Specify biome.json path
import-squeeze --config path/to/biome.json
```

### File Discovery

When no files are passed as arguments, import-squeeze reads `biome.json` to determine which files to process.

- Searches for `biome.json` (or `biome.jsonc`) from the current directory upward
- Reads `files.include` patterns (e.g. `["src/**", "lib/**"]`)
- Patterns prefixed with `!` are treated as excludes (e.g. `"!dist"`)
- Only `.ts`, `.tsx`, `.js`, `.jsx` files are processed

```jsonc
// biome.json
{
  "files": {
    "include": ["src/**", "!src/generated/**"]
  }
}
```

When files are passed directly (e.g. from lint-staged), biome.json is not read.

### Options

| Flag | Description |
|------|-------------|
| `--check` | Report files that need changes without modifying them. Exits with code 1 if any file needs squeezing. Useful for CI. |
| `--write` | Modify files in place. This is the default behavior. |
| `--config <path>` | Specify a custom path to `biome.json` instead of auto-detecting. |

## lint-staged

```json
{
  "lint-staged": {
    "*.{ts,tsx,js,jsx}": "import-squeeze"
  }
}
```

lint-staged passes changed files as arguments, so only staged files are processed — no full project scan.

## Supported Syntax

- `import { x } from 'y'`
- `import type { X } from 'y'`
- `import './side-effect'`
- `import.meta.glob(...)` (single & multiline)
- Multiline imports with `{ ... }`

Only ES `import` statements are handled. `require()` is not supported.

## How It Works

Line-based text processing — no AST parsing. Scans for import statements and removes blank lines between them while preserving everything else. Files are processed in parallel via [rayon](https://github.com/rayon-rs/rayon).

## License

MIT
