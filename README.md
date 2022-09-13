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
First, install the required dependecies:
### Ubuntu 22.04 or later
```
sudo apt install cargo libgtk-4-dev build-essential
```
### Fedora\RHEL
```
sudo dnf install cargo cairo-gobject-devel gdk-pixbuf2-devel python2-cairo-devel cairo-devel pango-devel graphene-devel cargo-devel
```
### Arch Linux
```
sudo pacman -S rust gtk4 base-devel
```
then run
```
cargo install ripdrag
```

# Usage
```
USAGE:
    ripdrag [OPTIONS] [PATHS]...

ARGS:
    <PATHS>...    Paths to the files you want to drag

OPTIONS:
    -a, --all                                Drag all the items together
    -A, --all-compact                        Show only the number of items and drag them together
    -d, --disable-thumbnails                 Don't load thumbnails from images
    -h, --content-height <CONTENT_HEIGHT>    Default height of the main window [default: 360]
        --help                               Print help information
    -i, --icons-only                         Only display icons, no labels
    -I, --from-stdin                         Accept paths from stdin
    -r, --resizable                          Make the window resizable
    -s, --icon-size <ICON_SIZE>              Size of icons and thumbnails [default: 32]
    -w, --content-width <CONTENT_WIDTH>      Min width of the main window [default: 360]
    -x, --and-exit                           Exit after first successful drag or drop
```
# TODO
There are still lots of thing to be done! Mainly:
- drag files from other apps to the terminal
- clean up code
- pacman, deb, rpm, windows and macos build scripts
- automated builds

Feel free to contribute ;)
