use std::fs;

use import_squeeze::{process_file, squeeze_imports, FileResult};

fn create_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn test_process_file_write_mode() {
    let dir = create_temp_dir();
    let file_path = dir.path().join("test.ts");
    fs::write(
        &file_path,
        "import { a } from 'a'\n\nimport { b } from 'b'\n\nconst x = 1\n",
    )
    .unwrap();

    let result = process_file(&file_path, false).unwrap();
    assert_eq!(result, FileResult::Changed);

    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "import { a } from 'a'\nimport { b } from 'b'\n\nconst x = 1\n");
}

#[test]
fn test_process_file_check_mode() {
    let dir = create_temp_dir();
    let file_path = dir.path().join("test.ts");
    let original = "import { a } from 'a'\n\nimport { b } from 'b'\n\nconst x = 1\n";
    fs::write(&file_path, original).unwrap();

    let result = process_file(&file_path, true).unwrap();
    assert_eq!(result, FileResult::Changed);

    // File should NOT be modified in check mode
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, original);
}

#[test]
fn test_process_file_unchanged() {
    let dir = create_temp_dir();
    let file_path = dir.path().join("test.ts");
    fs::write(
        &file_path,
        "import { a } from 'a'\nimport { b } from 'b'\n\nconst x = 1\n",
    )
    .unwrap();

    let result = process_file(&file_path, false).unwrap();
    assert_eq!(result, FileResult::Unchanged);
}

#[test]
fn test_squeeze_complex_real_world() {
    let input = r#"// @ts-nocheck
/* eslint-disable */

import type { FC } from 'react'

import {
  useState,
  useEffect,
  useCallback,
} from 'react'

import { Button } from '@/components/ui/button'

import { cn } from '@/lib/utils'

import './styles.css'

export const MyComponent: FC = () => {
  const [count, setCount] = useState(0)
  return <Button>{count}</Button>
}
"#;

    let expected = r#"// @ts-nocheck
/* eslint-disable */

import type { FC } from 'react'
import {
  useState,
  useEffect,
  useCallback,
} from 'react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import './styles.css'

export const MyComponent: FC = () => {
  const [count, setCount] = useState(0)
  return <Button>{count}</Button>
}
"#;

    assert_eq!(squeeze_imports(input), expected);
}

#[test]
fn test_biome_config_file_discovery() {
    let dir = create_temp_dir();

    // Create biome.json
    fs::write(
        dir.path().join("biome.json"),
        r#"{"files": {"include": ["src/**"]}}"#,
    )
    .unwrap();

    // Create src directory with files
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/test.ts"),
        "import { a } from 'a'\n\nimport { b } from 'b'\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("src/test.txt"),
        "not a ts file\n",
    )
    .unwrap();

    let config_path = dir.path().join("biome.json");
    let content = fs::read_to_string(&config_path).unwrap();
    let config = import_squeeze::config::parse_biome_config(&content).unwrap();
    let files = import_squeeze::config::resolve_file_paths(&config, dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("test.ts"));
}
