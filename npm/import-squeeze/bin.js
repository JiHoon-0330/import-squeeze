#!/usr/bin/env node

const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const os = require("os");

const PLATFORMS = {
  darwin: {
    arm64: "@import-squeeze/darwin-arm64",
  },
  linux: {
    x64: "@import-squeeze/linux-x64",
  },
};

function getBinaryPath() {
  const platform = os.platform();
  const arch = os.arch();

  const platformPackages = PLATFORMS[platform];
  if (!platformPackages) {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  const packageName = platformPackages[arch];
  if (!packageName) {
    throw new Error(`Unsupported architecture: ${platform}-${arch}`);
  }

  const binaryName = "import-squeeze";

  try {
    const packageDir = path.dirname(require.resolve(`${packageName}/package.json`));
    return path.join(packageDir, binaryName);
  } catch {
    throw new Error(
      `Could not find binary package ${packageName}. ` +
        `Make sure it's installed (it should be an optionalDependency).`
    );
  }
}

try {
  const binaryPath = getBinaryPath();
  try { fs.chmodSync(binaryPath, 0o755); } catch {}
  const result = execFileSync(binaryPath, process.argv.slice(2), {
    stdio: "inherit",
  });
} catch (error) {
  if (error.status !== undefined) {
    process.exit(error.status);
  }
  console.error(error.message);
  process.exit(1);
}
