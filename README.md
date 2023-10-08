# browser-pdf-print
uses webdriver api to convert any supported file to a pdf (useful for html -> pdf conversions)
WARN: only linux is supported for now, but if you have a use case get in touch with me, it would be fairly simple to add  both Mac and Windows support.

# usage
```bash
browser-pdf-print --source-file ~/Documents/basic.html --out-file /tmp/basic.pdf
```

for more information check out
```bash
browser-pdf-print --help
```

# installation
you need rust toolchain available under https://rustup.rs/
then it's as simple as running 
```bash
cargo install --git https://github.com/Niedzwiedzw/browser-pdf-print
```
