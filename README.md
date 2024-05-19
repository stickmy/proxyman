## Proxyman

[![Release](https://github.com/stickmy/proxyman/actions/workflows/release.yml/badge.svg)](https://github.com/stickmy/proxyman/actions/workflows/release.yml)

A http network debugging tool written by rust.

![connections](./screenshots/main.png)
![rules](./screenshots/rules.png)

## Features

- Support http1, http2, https connections.
- Support request redirect, request delay, modify response.


## Platform support

Support macos(x64, aarch64) only, the Windows is not supported currently.

## Rules usages

### Redirect

Each `Redirect` rule is split by a line and divided into two parts by spaces, the first part being the `regex` to be matched and the second part being the final address to be redirected.

**example**

```text
https://proxyhttpbin.org/(.*) https://httpbin.org/$1
```

### Delay

The format is as same as `Redirect` rule, the difference is the second part is the delaying milliseconds.

**example**
```text
# the-uri delaying-milliseconds
https://uri.com 200
```

### Troubleshooting

- If you get error messages such as broken dmg files with Apple Silicon machines. Please enter the following command in terminal and restart proxyman.

```sh
sudo xattr -d com.apple.quarantine /Applications/proxyman.app
```