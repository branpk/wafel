use core::fmt;

use wafel_data_access::MemoryLayout;
use wafel_data_type::Address;
use wafel_memory::MemoryRead;

use crate::SM64DataError;

/// A wrapper indicating a segmented address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Seg(pub Address);

impl fmt::Display for Seg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The SM64 segment table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegTable {
    /// Translation is a no-op (used by libsm64).
    Identity,
    /// The inner vec has 32 elements, each a 32 bit base address.
    Table(Vec<u32>),
}

impl SegTable {
    /// Convert a segmented address to a virtual address.
    pub fn seg_to_virt(&self, seg: Seg) -> Address {
        match self {
            SegTable::Identity => seg.0,
            SegTable::Table(seg_table) => {
                let addr = (seg.0).0 as u32;
                let segment = (addr & 0x1FFF_FFFF) >> 24;
                let offset = addr & 0x00FF_FFFF;

                let base = seg_table[segment as usize] | 0x8000_0000;
                let base = Address(base as usize);

                base + offset as usize
            }
        }
    }

    /// Convert a virtual address to a physical address (which can be used as a
    /// segmented address).
    pub fn virt_to_phys(&self, addr: Address) -> Seg {
        match self {
            SegTable::Identity => Seg(addr),
            SegTable::Table(_) => Seg(Address(((addr.0 as u32) & 0x1FFF_FFFF) as usize)),
        }
    }
}

/// Reads the segment table from the game for address translation.
pub fn read_seg_table(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
) -> Result<SegTable, SM64DataError> {
    // TODO: Handle libsm64

    let mut seg_table: Vec<u32> = vec![0; 32];
    let seg_table_addr = layout.symbol_address("sSegmentTable")?;
    memory.read_u32s(seg_table_addr, seg_table.as_mut_slice())?;

    Ok(SegTable::Table(seg_table))
}
