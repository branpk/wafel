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

To build from source, you'll need the following installed (be sure to use 64 bit versions):
* [Latest Rust nightly](https://www.rust-lang.org/tools/install)
* [Python 3.8 or above](https://www.python.org/downloads/)
* [pipenv](https://pipenv.pypa.io/en/latest/install/#installing-pipenv)
* [glslc](https://github.com/google/shaderc#downloads) (you'll need to add this to your path manually)

Setup:
```
git clone git@github.com:branpk/wafel.git
cd wafel
pipenv --python "path\to\python.exe"
pipenv install --dev
```

Building:
```
pipenv run python build.py
```

Running (dev mode or non-dev mode):
```
pipenv run python run.py
pipenv run python run.py --nodev
```

Building an executable:
```
pipenv run python run.py dist
```
