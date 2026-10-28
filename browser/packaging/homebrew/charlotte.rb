## Homebrew Cask template for Charlotte.
##
## This file is NOT wired into any tap yet. Homebrew resolves casks by
## looking in a repo named `homebrew-<tapname>` (for example,
## `EMN90909/homebrew-charlotte`), so this only becomes `brew install
## --cask charlotte` once that tap repo exists and this file (with the
## placeholders below filled in) is copied into its `Casks/` folder.
## See ../README.md for the full setup note.
##
## The release workflow (.github/workflows/release-browser.yml, at the
## repo root) writes a `checksums/darwin-checksums.txt` file to every
## GitHub Release it creates, which has the sha256 you need for the
## `sha256` line below.
##
## This is a template, not a published cask. macOS release packaging is
## not enabled in the repository CI until signing credentials are configured.

cask "charlotte" do
  version "0.3.0"
  sha256 "REPLACE_WITH_SHA256_FROM_RELEASE_CHECKSUMS_TXT"

  url "https://github.com/EMN90909/Charlotte/releases/download/browser-v#{version}/Charlotte-darwin-arm64-#{version}.zip"
  name "Charlotte"
  desc "Noethric-branded browser with a built-in chat room for every page"
  homepage "https://github.com/EMN90909/Charlotte"

  depends_on arch: :arm64

  app "Charlotte.app"

  zap trash: [
    "~/Library/Application Support/Charlotte",
    "~/Library/Preferences/com.charlotte.browser.plist",
    "~/Library/Saved Application State/com.charlotte.browser.savedState",
  ]
end
