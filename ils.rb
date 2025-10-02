class Ils < Formula
  desc "Interactive ls - A fast, interactive file browser for the terminal"
  homepage "https://github.com/yourusername/ils"
  url "https://github.com/yourusername/ils/archive/v0.1.0.tar.gz"
  sha256 "YOUR_SHA256_HERE"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release"
    bin.install "target/release/ils-bin"

    # Install default config files
    (buildpath/"keybindings.toml").install if (buildpath/"keybindings.toml").exist?
  end

  def post_install
    # Run the install routine
    system "#{bin}/ils-bin", "--install"
  end

  def caveats
    <<~EOS
      ils has been installed!

      The shell function has been added to your ~/.zshrc or ~/.bashrc
      Restart your shell or run:
        source ~/.zshrc  # or ~/.bashrc

      Then simply run 'ils' to start browsing!

      Configuration files are in ~/.config/ils/
    EOS
  end

  test do
    system "#{bin}/ils-bin", "--version"
  end
end
