# Homebrew's cask, for a tap of one's own (packaging/README.md):
# brew install --cask gustavn64/tap/openwgt
cask "openwgt" do
  version "0.13.0"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  url "https://github.com/gustavn64/OpenWGT/releases/download/v#{version}/openwgt-macos.dmg"
  name "OpenWGT"
  desc "Open engine for Maxis' Widget Workshop"
  homepage "https://gustavn.se/openwgt"

  app "OpenWGT.app"

  # Signed ad hoc only: Gatekeeper blocks the first start.
  caveats "OpenWGT is signed ad hoc, so macOS blocks its first start; then " \
          "System Settings > Privacy & Security > Open Anyway lets it run."
end
