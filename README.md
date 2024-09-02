# What is ripdrag?
![Crates.io](https://img.shields.io/crates/d/ripdrag?style=for-the-badge)
![GitHub top language](https://img.shields.io/github/languages/top/nik012003/ripdrag?color=dea584&style=for-the-badge)
![Crates.io](https://img.shields.io/crates/v/ripdrag?style=for-the-badge)

ripdrag is an application that lets you drag and drop files from and to the terminal.

It's designed to be feature to feature* compatible with [dragon](https://github.com/mwh/dragon), while being written in modern Rust and GTK4.

https://user-images.githubusercontent.com/10795335/189587471-7ed26f71-3f5e-4d8d-8048-7539e429531f.mp4

*some features like --on-top can't be ported over because of limitations in gtk4
# Use cases

Many applications expect files to be dragged into them. Normally you would have to put your beloved terminal aside and open a file manager to that, but now you can just type ```ripdrag FILENAME``` and be done.

Used in combination with a fuzzy finder like [fzf](https://github.com/junegunn/fzf) - e.g. ```ripdrag $(fzf)``` - can make for an amazingly quick and painless terminal experience.

# Installation
<details>
  <summary>Click to see the installation guide</summary>

### Install the required dependencies
#### Ubuntu 22.04 or later
```
sudo apt install libgtk-4-dev build-essential curl
curl --proto '=https' --tlsv1.3 https://sh.rustup.rs -sSf | sh
```
#### Fedora\CentOS\RHEL 
```
sudo dnf install cargo gdk-pixbuf2-devel pango-devel graphene-devel cairo-gobject-devel cairo-devel python2-cairo-devel gtk4-devel
```
#### Arch Linux
ripdrag is on the AUR: [ripdrag-git](https://aur.archlinux.org/packages/ripdrag-git)

If you want to install it manually, you need to install the requirements:
```
sudo pacman -Sy --needed rust gtk4 base-devel
```
#### MacOS
You need to have [homebrew](https://brew.sh) installed.
```
brew install rustup gtk4
rustup-init
```
#### NetBSD
A pre-compiled binary is available from the official repositories. To install it simply run,
```
pkgin install ripdrag
```

### Install the binary
(Do not use sudo, if you don't want it to be installed on root)
```
cargo install ripdrag
```
### Add cargo to path
(Not added by default)
```
PATH=$PATH:~/.cargo/bin
```

</details>

# Usage
```
Usage: ripdrag [OPTIONS] [PATH]...

Arguments:
  [PATH]...  Paths to the files you want to drag

Options:
  -v, --verbose                  Be verbose
  -t, --target                   Act as a target instead of source
  -k, --keep                     With --target, keep files to drag out
  -r, --resizable                Make the window resizable
  -x, --and-exit                 Exit after first successful drag or drop
  -i, --icons-only               Only display icons, no labels
  -d, --disable-thumbnails       Don't load thumbnails from images
  -s, --icon-size <SIZE>         Size of icons and thumbnails [default: 32]
  -W, --content-width <WIDTH>    Min width of the main window [default: 360]
  -H, --content-height <HEIGHT>  Default height of the main window [default: 360]
  -I, --from-stdin               Accept paths from stdin
  -a, --all                      Show a drag all button
  -A, --all-compact              Show only the number of items and drag them together
  -n, --no-click                 Don't open files on click
  -b, --basename                 Always show basename of each file
  -h, --help                     Print help
  -V, --version                  Print version
```

# TODO
There are still lots of thing to be done! Mainly:
- clean up code
- pacman, deb, rpm, windows and macos build scripts
- automated builds

Feel free to contribute ;)
