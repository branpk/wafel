# Wafel

Wafel is a TASing tool for Super Mario 64. It provides the ability to move backward and forward
through a TAS without using save states, and displays relevant in-game data variables.

Wafel is not an emulator, but it does require a vanilla SM64 ROM to operate.

This project is still a work in progress. The goal is to eventually replace Mupen64 as the
primary tool for vanilla SM64 TASing, but the feature set is not quite there yet.

Thanks to [Thestickman391](https://github.com/thestickman391) for contributing the Wafel logo.

Wafel is a backronym for **W**e **A**re **F**inally **E**mulator-**L**ess.

## Game versions

The following SM64 game versions are supported:
* JP: the original Japanese release
* US: the original US release
* EU: aka PAL (experimental, known crashes)
* SH: Shindou (experimental)

If you would like to use Wafel on an SM64 source hack, reach out and we can try to set it up.

## Downloading

Currently Windows is the only supported platform (but support for other platforms in the future
is likely).

You can download the latest version from the [Releases page](https://github.com/branpk/wafel/releases).

## Bugs and feature requests

Please log bugs and feature requests on Github [here](https://github.com/branpk/wafel/issues/new).
This increases visibility for other users, and I'm more likely to see them than on Discord.

## Wafel as a library

Wafel exposes a mid-level SM64 API that is intended to be useful for scripting / brute forcing.
This API is still fairly basic, but it can be extended with more useful features over time.

```python
import wafel

game = wafel.Game("libsm64/sm64_us.dll")

power_on = game.save_state()

game.advance_n(1500)
assert game.read("gCurrLevelNum") == game.constant("LEVEL_BOWSER_1")

game.load_state(power_on)
assert game.frame() == 0

for frame in range(0, 1000):
    if frame % 2 == 1:
        game.write("gControllerPads[0].button", game.constant("START_BUTTON"))
    game.advance()

game.advance_n(500)
assert game.read("gCurrLevelNum") == game.constant("LEVEL_CASTLE_GROUNDS")
```

Installing:
- For Python 3.7-3.9, download the latest Wafel release, and run `pip install --find-links=bindings/python wafel`.
- For Rust, include the dependency `wafel_api = { git = "https://github.com/branpk/wafel" }` (requires nightly).

Rust documentation is available [here](https://branpk.github.io/wafel/docs/dev/wafel_api/).
For Python, documentation is included in `wafel.pyi`, but you may also need to reference the
Rust documentation.

Wafel's low level code (beneath `wafel_api`) is also split up into Rust crates so that it can be
used in other SM64 projects if needed.

## Building

To build from source, you'll need the following installed (be sure to use 64 bit versions):
* [Latest Rust nightly](https://www.rust-lang.org/tools/install)
* [Python 3.8 or above](https://www.python.org/downloads/)
* [pipenv](https://pipenv.pypa.io/en/latest/install/#installing-pipenv)
* [Visual C++ 2015 run-time](https://www.microsoft.com/en-us/download/details.aspx?id=52685)

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
