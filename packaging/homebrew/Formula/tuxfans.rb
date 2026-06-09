class Tuxfans < Formula
  desc "Fan curve controller for TUXEDO laptops"
  homepage "https://github.com/Okazakee/tuxfans"
  license "MIT"

  url "https://github.com/Okazakee/tuxfans/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "1ade4f56e58544cdb94a1b111db4a15c78f3d6e780ccab94ba6dbe353b53eb9c"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "tuxfans", shell_output("#{bin}/tuxfans 2>&1", 1)
  end
end
