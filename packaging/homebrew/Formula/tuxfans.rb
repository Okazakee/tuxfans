class Tuxfans < Formula
  desc "Fan curve controller for TUXEDO laptops"
  homepage "https://github.com/Okazakee/tuxfans"
  license "MIT"

  url "https://github.com/Okazakee/tuxfans/archive/refs/tags/v0.3.0.tar.gz"
  sha256 "bfaa4adcf32b18628b7055123b0236c81904da03de6f1a036294c805bc3fa4fc"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "tuxfans", shell_output("#{bin}/tuxfans 2>&1", 1)
  end
end
