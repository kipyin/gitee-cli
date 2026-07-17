# Homebrew formula template for gitee-cli.
#
# version and sha256 values are placeholders. Regenerate or update this formula
# when cutting a release (e.g. from release artifacts and checksums).
#
# pkg-url pattern matches [package.metadata.binstall] in Cargo.toml:
#   {repo}/releases/download/v{version}/gitee-{target}-v{version}{archive-suffix}

class Gitee < Formula
  desc "A gh-like command-line client for Gitee"
  homepage "https://gitee.com/oschina/gitee-cli"
  version "PLACEHOLDER"
  license "MIT"

  # macOS only — selects the GitHub Release tarball for the host architecture.
  # The binary is statically linked enough for macOS; no runtime deps required.
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/oschina/gitee-cli/releases/download/v#{version}/gitee-aarch64-apple-darwin-v#{version}.tar.xz"
      sha256 "PLACEHOLDER_AARCH64_APPLE_DARWIN"
    else
      url "https://github.com/oschina/gitee-cli/releases/download/v#{version}/gitee-x86_64-apple-darwin-v#{version}.tar.xz"
      sha256 "PLACEHOLDER_X86_64_APPLE_DARWIN"
    end
  end

  def install
    bin.install Dir["gitee-*/gitee"].first
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gitee --version")
  end
end
