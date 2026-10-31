#!/usr/bin/env node

import { execSync } from "node:child_process";
import path from "node:path";

try {
  const output = execSync("cargo tarpaulin", {
    encoding: "utf-8",
    cwd: path.join(process.cwd(), "src-tauri"),
  }).trim();

  // Print output starting from "Coverage Results:" for visibility
  const coverageResultsIdx = output.indexOf("Coverage Results:");
  if (coverageResultsIdx !== -1) {
    console.log(output.slice(coverageResultsIdx));
  }

  // Extract the coverage percentage from the output
  const match = output.match(/(\d+\.\d+)% coverage/);

  let result = "fail";

  if (match) {
    const coverage = parseFloat(match[1]);
    console.log(`Native coverage: ${coverage}%`);

    // Target: 92% coverage for code testable in native environment
    // This accounts for WASM-only bindings in detector.rs that require browser APIs
    // Pure modules (preprocessing, postprocessing, types, utils) have near 100% coverage
    if (coverage >= 92.0) {
      result = "pass";
    } else {
      console.error(`Coverage ${coverage}% is below threshold of 92%`);
    }
  } else {
    console.error("Could not extract coverage percentage from output");
  }

  console.log("Test coverage " + result);
} catch (error) {
  console.error("Error running tarpaulin:", error.message);
  console.log("Test coverage fail");
}
