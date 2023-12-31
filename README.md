![screenshot](./screenshot/gossipbox.png)

[中文文档](./README.zh-CN.md)

#### Introduction
It's a chat software running in a `p2p` network. The program is written using `Slint-UI` and `Rust`.

#### Features
- [x] Supports automatic node discovery
- [x] Supports refreshing the list
- [x] Supports English and Chinese interfaces
- [x] Supports sending text
- [x] Supports sending images
- [x] Supports sending files

#### Note
- **Not supported** saving session history
- Images and files are not transferred using a P2P network, but using `tcp socket`

#### How to build?
- Install `Rust` and `Cargo`
- Execute `make build`
- Learn more in the [Makefile](./Makefile)

#### References
- [Slint Language Documentation](https://slint-ui.com/releases/1.0.0/docs/slint/)
- [github/slint-ui](https://github.com/slint-ui/slint)
- [Viewer for Slint](https://github.com/slint-ui/slint/tree/master/tools/viewer)
- [LSP (Language Server Protocol) Server for Slint](https://github.com/slint-ui/slint/tree/master/tools/lsp)
- [docs.libp2p.io](https://docs.libp2p.io/concepts/introduction/overview)
