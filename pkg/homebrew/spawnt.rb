# Homebrew formula for Spawnt — https://spawntv.vercel.app
# This formula targets the CLI binary for macOS (Intel + Apple Silicon).
#
# IMPORTANT: This is a pre-release stub. Do not publish until:
#   1. A signed v1.0 release tarball is available at spawntv.vercel.app/downloads.
#   2. Real SHA-256 checksums are substituted below.
#   3. macOS builds are verified on both x86_64 and arm64 hardware.
#   4. Legal review of the EULA is complete.
#
# Hosting: https://github.com/mwahaj36/homebrew-tap
# Install (future): brew install mwahaj36/tap/spawnt

class Spawnt < Formula
  desc "Mission control for AI coding agents"
  homepage "https://spawntv.vercel.app"
  version "PLACEHOLDER"
  license "BUSL-1.1"

  on_macos do
    on_intel do
      url "https://github.com/mwahaj36/spawnt-releases/releases/download/v#{version}/spawnt-v#{version}-darwin-x86_64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_DARWIN_X86_64"
    end
    on_arm do
      url "https://github.com/mwahaj36/spawnt-releases/releases/download/v#{version}/spawnt-v#{version}-darwin-aarch64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_DARWIN_AARCH64"
    end
  end

  def install
    bin.install "spawnt"
  end

  test do
    system bin/"spawnt", "--version"
  end
end
