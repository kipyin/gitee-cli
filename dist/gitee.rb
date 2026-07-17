# Regenerate sha256 on the next release:
# curl -sL https://github.com/kipyin/gitee-cli/releases/download/vVERSION/gitee-aarch64-apple-darwin-vVERSION.tar.xz | shasum -a 256
# curl -sL https://github.com/kipyin/gitee-cli/releases/download/vVERSION/gitee-x86_64-apple-darwin-vVERSION.tar.xz | shasum -a 256

class Gitee < Formula
  desc "A gh-like command-line client for Gitee"
  homepage "https://github.com/kipyin/gitee-cli"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kipyin/gitee-cli/releases/download/v#{version}/gitee-aarch64-apple-darwin-v#{version}.tar.xz"
      sha256 "5688a672b5458883907a479d3dd056436076dbbfaa44482e396752842c4a8c8b"
    else
      url "https://github.com/kipyin/gitee-cli/releases/download/v#{version}/gitee-x86_64-apple-darwin-v#{version}.tar.xz"
      sha256 "8d4f4a3ec47cd4cbcdb46ed0261b1610e01b43b8cc049361c292826dd33a1542"
    end
  end

  def install
    arch = Hardware::CPU.arm? ? "aarch64" : "x86_64"
    cd "gitee-#{arch}-apple-darwin-v#{version}" do
      bin.install "gitee"
    end
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gitee --version")
  end
end
