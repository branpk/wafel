"""Python bindings to Wafel's SM64 API.

See [wafel_api](https://branpk.github.io/wafel/docs/dev/wafel_api/) for full documentation.
"""

from typing import Optional, Dict, List, Tuple

class WafelError:
    """A Wafel API error."""

class Address:
    """A memory address."""

    def is_null(self) -> bool:
        """Return true if the address is equal to zero."""

class Surface:
    """An SM64 surface (currently missing several fields)."""

    @property
    def normal(self) -> List[float]:
        """The surface's normal vector."""
    @property
    def vertices(self) -> List[List[int]]:
        """The surface's vertex coordinates."""

class ObjectHitbox:
    """Hitbox information for an SM64 object."""

    @property
    def pos(self) -> List[float]:
        """The object's position (oPosX, oPosY, oPosZ)."""
    @property
    def hitbox_height(self) -> float:
        """The object's hitbox height (hitboxHeight)."""
    @property
    def hitbox_radius(self) -> float:
        """The object's hitbox radius (hitboxRadius)."""

class Game:
    """An SM64 API that uses a traditional frame advance / save state model."""

    def __init__(self, dll_path: str) -> None:
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
    def rerecords(self) -> int:
        """Return the number of times that a save state has been loaded."""
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

    def frame(self) -> int:
        """Return the frame that the save state was token on."""

class M64Metadata:
    """Metadata for a .m64 TAS."""

    def __init__(self, crc_code: int, country_code: int) -> None:
        """Create a new metadata object with the given CRC and country code."""
    @staticmethod
    def with_version(version: str) -> "M64Metadata":
        """Create a new metadata object using the CRC and country code for the given SM64 version.

        Version strings: "jp", "us", "eu", "sh"
        """
    def crc_code(self) -> int:
        """Get the CRC code."""
    def set_crc_code(self, crc_code: int) -> None:
        """Set the CRC code."""
    def country_code(self) -> int:
        """Get the country code."""
    def set_country_code(self, country_code: int) -> None:
        """Set the country code."""
    def version(self) -> Optional[str]:
        """Return the SM64 version with matching CRC and country code, if it exists.

        Version strings: "jp", "us", "eu", "sh"
        """
    def set_version(self, version: str) -> None:
        """Set the CRC and country code to match the given SM64 version.

        Version strings: "jp", "us", "eu", "sh"
        """
    def author(self) -> str:
        """Get the author field."""
    def set_author(self, author: str) -> None:
        """Set the author field (max 222 bytes)."""
    def description(self) -> str:
        """Get the description field."""
    def set_description(self, description: str) -> None:
        """Set the description field (max 256 bytes)."""
    def rerecords(self) -> int:
        """Get the number of rerecords."""
    def set_rerecords(self, rerecords: int) -> None:
        """Set the number of rerecords."""
    def add_rerecords(self, rerecords: int) -> None:
        """Add a number of rerecords, saturating on overflow."""

class Input:
    """A set of inputs for a given frame."""

    def __init__(self, buttons: int, stick_x: int, stick_y: int) -> None:
        """Create an Input with the given button flags and stick coordinates."""
    @property
    def buttons(self) -> int:
        """The standard button bit flags."""
    @buttons.setter
    def buttons(self, buttons: int) -> None:
        pass
    @property
    def stick_x(self) -> int:
        """The joystick x coordinate."""
    @stick_x.setter
    def stick_x(self, stick_x: int) -> None:
        pass
    @property
    def stick_y(self) -> int:
        """The joystick y coordinate."""
    @stick_y.setter
    def stick_y(self, stick_y: int) -> None:
        pass

def load_m64(filename: str) -> Tuple[M64Metadata, List[Input]]:
    """Load an m64 TAS from a file."""

def save_m64(filename: str, metadata: M64Metadata, inputs: List[Input]) -> None:
    """Save an m64 TAS to a file."""
