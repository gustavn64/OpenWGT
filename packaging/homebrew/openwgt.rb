# Homebrew's cask, for a tap of one's own (packaging/README.md):
# brew install --cask gustavn64/tap/openwgt
cask "openwgt" do
  version "0.11.0"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  url "https://github.com/gustavn64/OpenWGT/releases/download/v#{version}/openwgt-macos.dmg"
  name "OpenWGT"
  desc "Open engine for Maxis' Widget Workshop"
  homepage "https://gustavn.se/openwgt"

  app "OpenWGT.app"

  # Signed ad hoc only: Gatekeeper asks once.
  caveats "OpenWGT is signed ad hoc; the first start needs a right click, Open."
end
