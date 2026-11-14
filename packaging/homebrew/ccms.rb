class Ccms < Formula
  desc "High-performance CLI for searching Claude Code session JSONL files"
  homepage "https://github.com/mkusaka/ccms"
  version "__VERSION__"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "__DARWIN_ARM64_URL__"
      sha256 "__DARWIN_ARM64_SHA256__"
    else
      url "__DARWIN_X64_URL__"
      sha256 "__DARWIN_X64_SHA256__"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "__LINUX_ARM64_URL__"
      sha256 "__LINUX_ARM64_SHA256__"
    else
      url "__LINUX_X64_URL__"
      sha256 "__LINUX_X64_SHA256__"
    end
  end

  def install
    bin.install "ccms"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/ccms --version")
  end
end
