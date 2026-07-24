#!/usr/bin/env python3
"""Write Formula/gitee.rb for a release's macOS tarball digests.

Usage:
  bump-homebrew-formula.py <formula-path> <version> <arm64-sha256> <x86_64-sha256>
"""

from __future__ import annotations

import sys
from pathlib import Path


def main() -> None:
    if len(sys.argv) != 5:
        print(
            "usage: bump-homebrew-formula.py <formula-path> <version> "
            "<arm64-sha256> <x86_64-sha256>",
            file=sys.stderr,
        )
        sys.exit(2)

    path = Path(sys.argv[1])
    version, arm_sha, x86_sha = sys.argv[2], sys.argv[3], sys.argv[4]

    path.write_text(
        f"""# Regenerate sha256 on the next release:
# curl -sL https://github.com/kipyin/gitee-cli/releases/download/vVERSION/gitee-aarch64-apple-darwin-vVERSION.tar.xz | shasum -a 256
# curl -sL https://github.com/kipyin/gitee-cli/releases/download/vVERSION/gitee-x86_64-apple-darwin-vVERSION.tar.xz | shasum -a 256

class Gitee < Formula
  desc "A gh-like command-line client for Gitee"
  homepage "https://github.com/kipyin/gitee-cli"
  version "{version}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kipyin/gitee-cli/releases/download/v#{{version}}/gitee-aarch64-apple-darwin-v#{{version}}.tar.xz"
      sha256 "{arm_sha}"
    else
      url "https://github.com/kipyin/gitee-cli/releases/download/v#{{version}}/gitee-x86_64-apple-darwin-v#{{version}}.tar.xz"
      sha256 "{x86_sha}"
    end
  end

  def install
    # Homebrew auto-cds into the single top-level dir the tarball extracts to,
    # so the binary is already in the cwd here.
    bin.install "gitee"
  end

  test do
    assert_match version.to_s, shell_output("#{{bin}}/gitee --version")
  end
end
"""
    )


if __name__ == "__main__":
    main()
