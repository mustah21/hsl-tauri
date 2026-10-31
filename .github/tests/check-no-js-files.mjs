#!/usr/bin/env node

/**
 * Verifies that there are no JavaScript files in the root of the project and in the src directory.
 */

import fs from "node:fs";
import path from "node:path";
import { cwd } from "node:process";

const rootDir = cwd();
const srcDirs = ["src", "src-tauri"];

// Find JavaScript files in a directory, optionally recursive
function findJsFiles(dir, recursive = false) {
  const files = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory() && recursive) {
      files.push(...findJsFiles(fullPath, recursive));
    } else if (entry.isFile() && (entry.name.endsWith(".js") || entry.name.endsWith(".mjs"))) {
      files.push(fullPath);
    }
  }

  return files;
}

// Check that there are no JavaScript files in src directory (recursive)
console.log("Checking for JavaScript files in source directories...");
const jsFiles = [];

srcDirs.forEach((srcDir) => {
  jsFiles.push(...findJsFiles(path.join(rootDir, srcDir), true));
});

if (jsFiles.length > 0) {
  console.error(`Found ${jsFiles.length} JavaScript file(s) in source directories:`);
  jsFiles.forEach((file) => {
    console.error(`  - ${file.replace(rootDir + path.sep, "")}`);
  });
  console.error("The whole UI should be implemented in Rust, not JavaScript");
  process.exit(1);
}
console.log("No JavaScript files in source directories");

process.exit(0);
