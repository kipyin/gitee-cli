# Regenerate sha256 on the next release:
# curl -sL https://github.com/kipyin/gitee-cli/releases/download/vVERSION/gitee-aarch64-apple-darwin-vVERSION.tar.xz | shasum -a 256
# curl -sL https://github.com/kipyin/gitee-cli/releases/download/vVERSION/gitee-x86_64-apple-darwin-vVERSION.tar.xz | shasum -a 256

class Gitee < Formula
  desc "A gh-like command-line client for Gitee"
  homepage "https://github.com/kipyin/gitee-cli"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kipyin/gitee-cli/releases/download/v#{version}/gitee-aarch64-apple-darwin-v#{version}.tar.xz"
      sha256 "829c416bd980c8040c60d97de74ba10b6c29b8dc4db5fdd381f9fff7808fae52"
    else
      url "https://github.com/kipyin/gitee-cli/releases/download/v#{version}/gitee-x86_64-apple-darwin-v#{version}.tar.xz"
      sha256 "6a71817d7abea98e6052a4e1a692555ef44223ba37dc3111cb937e071189370c"
    end
  end

  def install
    # Homebrew auto-cds into the single top-level dir the tarball extracts to,
    # so the binary is already in the cwd here.
    bin.install "gitee"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gitee --version")
  end
end
