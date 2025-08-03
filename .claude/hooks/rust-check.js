#!/usr/bin/env node

const { execSync } = require('child_process');
const fs = require('fs');

function runCommand(command, cwd = process.env.CLAUDE_PROJECT_DIR || '.') {
  try {
    const output = execSync(command, {
      cwd,
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe']
    });
    return { success: true, output };
  } catch (error) {
    return {
      success: false,
      code: error.status || 1,
      stdout: error.stdout?.toString() || '',
      stderr: error.stderr?.toString() || ''
    };
  }
}

function main() {
  // Read input from stdin
  let input = '';
  try {
    input = fs.readFileSync(0, 'utf-8');
  } catch (e) {
    // Ignore if no stdin
  }

  const projectDir = process.env.CLAUDE_PROJECT_DIR || '.';

  // Run cargo fmt
  runCommand('cargo fmt', projectDir);

  // Try to auto-fix with clippy
  runCommand('cargo clippy --fix --allow-dirty --allow-staged 2>&1', projectDir);

  // Check for remaining errors
  const checkResult = runCommand('cargo clippy -- -D warnings 2>&1', projectDir);

  if (!checkResult.success) {
    // Parse error output
    const output = checkResult.stderr + checkResult.stdout;
    const errorLines = output.split('\n')
      .filter(line => {
        const trimmed = line.trim();
        return trimmed.includes('error:') || 
               trimmed.includes('warning:') ||
               trimmed.includes('help:') ||
               trimmed.includes('-->') ||
               trimmed.startsWith('|');
      })
      .slice(0, 50); // Limit to 50 lines

    // Create structured output for Claude
    const result = {
      decision: 'block',
      reason: [
        'Rust code has clippy errors that need fixing:',
        '',
        ...errorLines,
        '',
        'Please fix these clippy errors in the code.'
      ].join('\n')
    };

    // Output JSON for Claude
    console.log(JSON.stringify(result));
    process.exit(0);
  }

  // Success
  process.exit(0);
}

main();