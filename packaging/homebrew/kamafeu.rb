class Kamafeu < Formula
  desc "Editor e sintetizador vocal baseado em amostragem e splicing"
  homepage "https://github.com/studiopomar/kamafeu"
  version "1.0.2-rc.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/studiopomar/kamafeu/releases/download/v#{version}/kamafeu-v#{version}-macos-arm64.dmg"
      sha256 "REPLACE_WITH_RELEASE_SHA256"
    else
      url "https://github.com/studiopomar/kamafeu/releases/download/v#{version}/kamafeu-v#{version}-macos-x86_64.dmg"
      sha256 "REPLACE_WITH_RELEASE_SHA256"
    end
  end

  on_linux do
    url "https://github.com/studiopomar/kamafeu/releases/download/v#{version}/kamafeu-v#{version}-linux-x64.tar.gz"
    sha256 "REPLACE_WITH_RELEASE_SHA256"
  end

  def install
    bin.install "kamafeu"
    bin.install "copaiba"
  end

  test do
    assert_match "kamafeu", shell_output("#{bin}/kamafeu --help")
  end
end
