class Ils < Formula
  desc "Interactive ls - A fast, keyboard-driven file browser for the terminal"
  homepage "https://github.com/jordannakamoto/ils"
  url "https://github.com/jordannakamoto/ils/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "f4641869a4dc914bb4edae2105e704a7f108d23c88b8273203d626d12e65c0c9"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    
    # Create shell integration script
    (prefix/"etc/ils_init.sh").write <<~EOS
      ils() {
        #{bin}/ils-bin "$@"
        if [ -f /tmp/ils_cd ]; then
          local target=$(cat /tmp/ils_cd)
          rm /tmp/ils_cd
          if [ -d "$target" ]; then
            cd "$target"
          else
            echo "$target"
          fi
        fi
      }
    EOS
  end

  def caveats
    <<~EOS
      To enable directory navigation, add this to your ~/.zshrc or ~/.bashrc:

        source $(brew --prefix)/opt/ils/etc/ils_init.sh

      Or run: echo 'source $(brew --prefix)/opt/ils/etc/ils_init.sh' >> ~/.zshrc
    EOS
  end

  test do
    system "#{bin}/ils-bin", "--version"
  end
end
