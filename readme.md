git-whence is an interactive version of git-blame that makes it easy
to reblame at a previous version and follow a line through commit history

`git whence src/git.rs`

![](https://user-images.githubusercontent.com/90059/237033030-3984c97c-b8b0-4cb4-989b-3b135b22c8ba.png)

`b` to reblame at a previous version

![](https://user-images.githubusercontent.com/90059/237034888-3785170c-d9d5-4c67-ad6b-3c0411ae0cb3.png)

`B` will pop the stack and blame at the initial version (first screenshot)

`<Enter>` to follow a line through commit history

![](https://user-images.githubusercontent.com/90059/237033938-08817c9b-44dd-4313-9ecb-f3ba89890beb.png)

press `h` for help

## installing

assuming `~/bin` is on your `PATH`,
```sh
cd ~/bin
curl -L https://github.com/raylu/git-whence/releases/latest/download/git-whence-$(uname -m | sed s/arm64/aarch64/)-$(uname -s | awk '{print tolower($0)}' | sed -e s/darwin/apple-darwin/ -e s/linux/unknown-linux-gnu/) -o git-whence
chmod +x git-whence
```

alternatively, download a binary from the [releases](https://github.com/raylu/git-whence/releases) page

or install from source via `cargo install git-whence`: https://crates.io/crates/git-whence

### macOS

```sh
brew tap raylu/formulae
brew install --cask git-whence
```

if you downloaded manually and get an error about how it "can’t be opened because Apple cannot check it 
or malicious software", this is because the quarantine extended attribute has been set by your browser.
either `xattr -d com.apple.quarantine git-whence` or use `curl`/`wget` to download instead
