"""Python bindings to Wafel's SM64 API.

See [wafel_api](https://branpk.github.io/wafel/docs/dev/wafel_api/) for full documentation.
"""

from typing import Optional, Dict, List

class WafelError:
    """A Wafel API error."""

class Address:
    """A memory address."""

class Surface:
    """An SM64 surface (currently missing several fields)."""

class ObjectHitbox:
    """Hitbox information for an SM64 object."""

class Game:
    """An SM64 API that uses a traditional frame advance / save state model."""

    @staticmethod
    def open(dll_path: str) -> "Game":
        """Load a libsm64 DLL."""
    def read(self, path: str) -> object:
        """Read a value from memory.

        See the [wafel_api crate documentation](https://branpk.github.io/wafel/docs/dev/wafel_api/)
        for the path syntax.
        """
    def read_string_at(self, address: Address) -> bytes:
        """Read a null terminated string from memory at the given address.

        See the [wafel_api crate documentation](https://branpk.github.io/wafel/docs/dev/wafel_api/)
        for the path syntax.
        """
    def address(self, path: str) -> Optional[Address]:
        """Find the address of a path.

        This method returns `None` if `?` is used in the path and the expression before
        `?` evaluates to a null pointer.

        See the [wafel_api crate documentation](https://branpk.github.io/wafel/docs/dev/wafel_api/)
        for the path syntax.
        """
    def address_to_symbol(self, address: Address) -> Optional[str]:
        """Return the name of the global variable at the given address.

        Returns None if no global variable is at the address.
        """
    def data_type(self, path: str) -> str:
        """Return a simplified description of the type of the given variable.

        See the [wafel_api crate documentation](https://branpk.github.io/wafel/docs/dev/wafel_api/)
        for the path syntax.
        """
    def write(self, path: str, value: object) -> None:
        """Write a value to memory.

        See the [wafel_api crate documentation](https://branpk.github.io/wafel/docs/dev/wafel_api/)
        for the path syntax.
        """
    def frame(self) -> int:
        """Get the frame of the current game state."""
    def advance(self) -> None:
        """Advance a single frame."""
    def advance_n(self, num_frames: int) -> None:
        """Advance multiple frames."""
    def save_state(self) -> "SaveState":
        """Create a save state using the current game state."""
    def load_state(self, state: "SaveState") -> None:
        """Load a save state."""
    def constant(self, name: str) -> object:
        """Return the value of the macro constant or enum variant with the given name."""
    def mario_action_names(self) -> Dict[int, str]:
        """Return a mapping from Mario action values to their name (e.g. `ACT_IDLE`)."""
    def frame_log(self) -> List[Dict[str, object]]:
        """Read the Wafel frame log for the previous frame advance."""
    def surfaces(self) -> List[Surface]:
        """Read the currently loaded surfaces."""
    def object_hitboxes(self) -> List[ObjectHitbox]:
        """Read the hitboxes for active objects."""

class SaveState:
    """A save state used by Game."""
