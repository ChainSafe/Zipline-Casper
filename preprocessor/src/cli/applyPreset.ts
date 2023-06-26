const network = valueOfArg("network");

// Translate network to preset
if (network) {
  if (network === "gnosis" || network === "chiado") {
    process.env.LODESTAR_PRESET = "gnosis";
  }
}

/**
 * Valid syntax
 * - `--network minimal`
 * - `--network=minimal`
 */
function valueOfArg(argName: string): string | null {
  // Syntax `--preset minimal`
  // process.argv = ["--preset", "minimal"];

  {
    const index = process.argv.indexOf(`--${argName}`);
    if (index > -1) {
      return process.argv[index + 1] ?? "";
    }
  }

  // Syntax `--preset=minimal`
  {
    const prefix = `--${argName}=`;
    const item = process.argv.find((arg) => arg.startsWith(prefix));
    if (item) {
      return item.slice(prefix.length);
    }
  }

  return null;
}

// Add empty export to make this a module
export {};
