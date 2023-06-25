## proxyman

[![Release](https://github.com/stickmy/proxyman/actions/workflows/release.yml/badge.svg)](https://github.com/stickmy/proxyman/actions/workflows/release.yml)

A http network debugging tool written by rust. 

## Features

- Support http1, http2, https connections.
- Support request redirect, request delay, modifiy response.


## Platform support

Support macos(x64, aarch64) only.

### Troubleshooting

- If you get error messages such as broken dmg files with Apple Silicon machines. Please enter the following command in terminal and restart proxyman.

```sh
sudo xattr -d com.apple.quarantine /Applications/proxyman.app
```