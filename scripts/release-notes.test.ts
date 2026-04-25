import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const script = resolve(__dirname, "release-notes.sh");

function run(input: string): string {
  const result = spawnSync("bash", [script, "--from-stdin"], {
    input,
    encoding: "utf-8",
  });
  if (result.status !== 0) {
    throw new Error(
      `release-notes.sh exited ${result.status}: ${result.stderr}`,
    );
  }
  return result.stdout;
}

describe("release-notes.sh", () => {
  it("emits Features and Bug fixes sections from Conventional Commits", () => {
    const out = run(
      [
        "abcd123|feat(cleanup): add auto-cleanup service",
        "ef45678|fix(player): handle remux failure",
        "9876543|chore: bump deps",
      ].join("\n"),
    );
    expect(out).toMatch(/## Features/);
    expect(out).toMatch(/auto-cleanup service \(abcd123\)/);
    expect(out).toMatch(/## Bug fixes/);
    expect(out).toMatch(/handle remux failure \(ef45678\)/);
    expect(out).toMatch(/## Other/);
    expect(out).toMatch(/bump deps \(9876543\)/);
  });

  it("emits Performance section when perf subjects are present", () => {
    const out = run(["abc1234|perf(sync): cache drift threshold"].join("\n"));
    expect(out).toMatch(/## Performance/);
    expect(out).toMatch(/cache drift threshold \(abc1234\)/);
  });

  it("treats refactor / test / docs / style / build / ci as Other", () => {
    const out = run(
      [
        "1111111|refactor(db): extract migrate helper",
        "2222222|test(cleanup): integration coverage",
        "3333333|docs: update CHANGELOG",
        "4444444|style: rustfmt sweep",
        "5555555|build(ci): bump actions/cache@v5",
        "6666666|ci(release): add release workflow",
      ].join("\n"),
    );
    expect(out).toMatch(/## Other/);
    for (const subject of [
      "extract migrate helper",
      "integration coverage",
      "update CHANGELOG",
      "rustfmt sweep",
      "bump actions/cache@v5",
      "add release workflow",
    ]) {
      expect(out).toContain(subject);
    }
    expect(out).not.toMatch(/## Uncategorised/);
  });

  it("buckets non-conforming subjects into Uncategorised", () => {
    const out = run(["aaaaaaa|WIP — drive-by typo"].join("\n"));
    expect(out).toMatch(/## Uncategorised/);
    expect(out).toMatch(/drive-by typo \(aaaaaaa\)/);
  });

  it("omits empty sections from output", () => {
    const out = run(["abcdef0|feat: only feature"].join("\n"));
    expect(out).toMatch(/## Features/);
    expect(out).not.toMatch(/## Bug fixes/);
    expect(out).not.toMatch(/## Performance/);
    expect(out).not.toMatch(/## Other/);
  });

  it("returns empty (no headings) for empty input", () => {
    const out = run("");
    expect(out).toBe("");
  });
});
