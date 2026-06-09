class Tuxfans < Formula
  desc "Fan curve controller for TUXEDO laptops"
  homepage "https://github.com/Okazakee/tuxfans"
  license "MIT"

  stable do
    url "https://github.com/Okazakee/tuxfans/archive/refs/tags/v0.1.0.tar.gz"
    sha256 "UNCOMMITTED"
  end

  head do
    url "https://github.com/Okazakee/tuxfans.git", branch: "main"
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "tuxfans", shell_output("#{bin}/tuxfans 2>&1", 1)
  end
end
