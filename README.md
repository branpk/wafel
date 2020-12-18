# Wafel

Wafel is a TASing tool for Super Mario 64. It provides the ability to move backward and forward
through a TAS without using save states, and displays relevant in-game data variables.

Wafel is not an emulator, but it does require a vanilla SM64 ROM to operate.

This project is still a work in progress. The goal is to eventually replace Mupen64 as the
primary tool for vanilla SM64 TASing, but the feature set is not quite there yet.

Currently Windows is the only supported platform (but support for other platforms in the future
is likely).

## Bugs and feature requests

Please log bugs and feature requests on Github [here](https://github.com/branpk/wafel/issues/new).
This increases visibility for other users, and I'm more likely to see them than on Discord.


## Building
Make sure you have [Python 3.8](https://www.python.org/downloads/) or above installed and [Rust nightly](https://www.rust-lang.org/tools/install). Rust nightly can be installed by running `rustup toolchain install nightly` once Rust has been successfully installed.

Clone or download this repository somewhere and run `pip install -r requirements.txt`. To build an exe, run `python build.py dist`. If you get an error such as 
```
Traceback (most recent call last):
  File "build.py", line 34, in <module>
    subprocess.run(
<snip>
FileNotFoundError: [WinError 2] The system cannot find the file specified
```
then move `%appdata%\Python\Python38\Scripts\pyinstaller.exe` to `C:\Python38\Scripts\pyinstaller.exe`

This repo uses [glslc](https://github.com/google/shaderc) by Google for building, licensed under the Apache License 2.0.
