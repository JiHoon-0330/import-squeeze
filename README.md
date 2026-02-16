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

## lint-staged

```json
{
  "lint-staged": {
    "*.{ts,tsx,js,jsx}": "import-squeeze"
  }
}
```

## Supported Syntax

- `import { x } from 'y'`
- `import type { X } from 'y'`
- `import './side-effect'`
- `import.meta.glob(...)` (single & multiline)
- Multiline imports with `{ ... }`

## How It Works

Line-based text processing — no AST parsing. Scans for import statements and removes blank lines between them while preserving everything else. Files are processed in parallel via [rayon](https://github.com/rayon-rs/rayon).

## License

MIT
