class Tuxfans < Formula
  desc "Fan curve controller for TUXEDO laptops"
  homepage "https://github.com/Okazakee/tuxfans"
  license "MIT"

  url "https://github.com/Okazakee/tuxfans/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "cbc7a5bafc59175abad3047a907be9f55642a0b119522cd93e5362627e4e565b"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "tuxfans", shell_output("#{bin}/tuxfans 2>&1", 1)
  end
end
