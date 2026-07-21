# Regenerate sha256 on the next release:
# curl -sL https://github.com/kipyin/gitee-cli/releases/download/vVERSION/gitee-aarch64-apple-darwin-vVERSION.tar.xz | shasum -a 256
# curl -sL https://github.com/kipyin/gitee-cli/releases/download/vVERSION/gitee-x86_64-apple-darwin-vVERSION.tar.xz | shasum -a 256

class Gitee < Formula
  desc "A gh-like command-line client for Gitee"
  homepage "https://github.com/kipyin/gitee-cli"
  version "0.1.6"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kipyin/gitee-cli/releases/download/v#{version}/gitee-aarch64-apple-darwin-v#{version}.tar.xz"
      sha256 "40559f63259a5720589d53050b0e2ee6c01ef2195108bef96d72dec835155702"
    else
      url "https://github.com/kipyin/gitee-cli/releases/download/v#{version}/gitee-x86_64-apple-darwin-v#{version}.tar.xz"
      sha256 "f460c1c596d5950b43987b720c0a82ec306af50464c6a678153843e1c0344595"
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
