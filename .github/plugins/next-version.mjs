import { writeFileSync } from "node:fs";

/**
 * Writes the version that semantic-release is about to release to a file.
 *
 * The ISO build needs the version before mkarchiso runs, but the `prepare` and
 * `publish` steps are skipped in `--dry-run` mode (`dryRun: false` in
 * semantic-release's plugin definitions). `verifyRelease` does run in dry-run
 * mode, which makes it the only hook that can expose `nextRelease.version`
 * before the release is published.
 */
export function verifyRelease(pluginConfig, { nextRelease, logger }) {
  writeFileSync(pluginConfig.outputFile, nextRelease.version);
  logger.log("Next release version: %s", nextRelease.version);
}
