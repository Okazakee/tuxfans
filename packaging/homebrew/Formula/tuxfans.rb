class Tuxfans < Formula
  desc "Fan curve controller for TUXEDO laptops"
  homepage "https://github.com/Okazakee/tuxfans"
  license "MIT"

  url "https://github.com/Okazakee/tuxfans/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "1db27a2ad44f1517f3c30ef997bd9504b6763c833962f3b4739e0c13f6b9745d"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "tuxfans", shell_output("#{bin}/tuxfans 2>&1", 1)
  end
end
