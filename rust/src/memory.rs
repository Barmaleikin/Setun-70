//! Level-1 memory of Setun-70 (description §2.5, §5.1).
//!
//! 27 pages of 81 syllables (trytes) each; syllable addresses −40..=40,
//! page numbers −13..=13:
//!   RAM (read/write): pages −4..=4                 (9 pages)
//!   ROM (read-only):  pages −13..=−5 and 5..=13    (18 pages)
//!
//! Level-2 memory (drum `f`) and peripheral buffers are not part of this
//! module; they arrive with the exchange and I/O layers.

use analemma::tryte::Tryte;

pub const PAGE_MIN: i8 = -13;
pub const PAGE_MAX: i8 = 13;
pub const ADDR_MIN: i8 = -40;
pub const ADDR_MAX: i8 = 40;
pub const PAGE_SIZE: usize = (ADDR_MAX - ADDR_MIN + 1) as usize; // 81

pub const RAM_MIN: i8 = -4;
pub const RAM_MAX: i8 = 4;

/// Number of level-1 pages.
pub const PAGE_COUNT: usize = (PAGE_MAX - PAGE_MIN + 1) as usize; // 27
/// Number of RAM pages.
pub const RAM_PAGE_COUNT: usize = (RAM_MAX - RAM_MIN + 1) as usize; // 9
/// Number of ROM pages (the rest).
pub const ROM_PAGE_COUNT: usize = PAGE_COUNT - RAM_PAGE_COUNT; // 18

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemError {
    /// Write to a read-only (ROM) page.
    ReadOnly { page: i8 },
    /// Page or syllable address outside the level-1 memory.
    BadAddress { page: i8, addr: i8 },
}

#[derive(Clone, Copy, Debug)]
pub struct Page {
    /// Indexed by `addr - ADDR_MIN`, i.e. slot 0 is syllable −40.
    pub syllables: [Tryte; PAGE_SIZE],
}

impl Page {
    pub fn zeroed() -> Self {
        Page { syllables: [Tryte::zero(); PAGE_SIZE] }
    }

    #[inline(always)]
    fn slot(addr: i8) -> Option<usize> {
        if (ADDR_MIN..=ADDR_MAX).contains(&addr) {
            Some((addr - ADDR_MIN) as usize)
        } else {
            None
        }
    }

    pub fn read(&self, addr: i8) -> Option<Tryte> {
        Self::slot(addr).map(|i| self.syllables[i])
    }

    pub fn write(&mut self, addr: i8, value: Tryte) -> bool {
        match Self::slot(addr) {
            Some(i) => { self.syllables[i] = value; true }
            None => false,
        }
    }
}

/// Level-1 memory: 27 pages × 81 syllables.
#[derive(Clone, Debug)]
pub struct Memory1 {
    pages: [Page; PAGE_COUNT],
}

impl Memory1 {
    pub fn zeroed() -> Self {
        Memory1 { pages: [Page::zeroed(); PAGE_COUNT] }
    }

    #[inline(always)]
    fn pslot(page: i8) -> Option<usize> {
        if (PAGE_MIN..=PAGE_MAX).contains(&page) {
            Some((page - PAGE_MIN) as usize)
        } else {
            None
        }
    }

    pub fn is_ram(page: i8) -> bool {
        (RAM_MIN..=RAM_MAX).contains(&page)
    }

    pub fn is_rom(page: i8) -> bool {
        Self::pslot(page).is_some() && !Self::is_ram(page)
    }

    pub fn page(&self, page: i8) -> Option<&Page> {
        Self::pslot(page).map(|i| &self.pages[i])
    }

    /// Read one syllable from any page (RAM or ROM).
    pub fn read(&self, page: i8, addr: i8) -> Option<Tryte> {
        Self::pslot(page).and_then(|i| self.pages[i].read(addr))
    }

    /// Write one syllable. Only RAM pages are writable.
    pub fn write(&mut self, page: i8, addr: i8, value: Tryte) -> Result<(), MemError> {
        if !Self::is_ram(page) {
            return Err(MemError::ReadOnly { page });
        }
        match Self::pslot(page) {
            Some(i) if self.pages[i].write(addr, value) => Ok(()),
            _ => Err(MemError::BadAddress { page, addr }),
        }
    }

    /// Reads `len` (1..=3) syllables whose SENIOR syllable sits at `addr_hi`
    /// (ascending addresses `addr_hi−len+1 ..= addr_hi`), returned senior-first.
    /// This is the operand shape of the address syllable (§2, REFSYL).
    pub fn read_syllables(&self, page: i8, addr_hi: i8, len: u8) -> Option<[Tryte; 3]> {
        if !(1..=3).contains(&len) {
            return None;
        }
        let mut out = [Tryte::zero(); 3];
        for i in 0..len as i8 {
            out[i as usize] = self.read(page, addr_hi - i)?;
        }
        Some(out)
    }

    /// Writes `len` (1..=3) syllables senior-first, senior at `addr_hi`.
    pub fn write_syllables(&mut self, page: i8, addr_hi: i8, len: u8, data: &[Tryte; 3]) -> Result<(), MemError> {
        if !(1..=3).contains(&len) {
            return Err(MemError::BadAddress { page, addr: addr_hi });
        }
        for i in 0..len as i8 {
            self.write(page, addr_hi - i, data[i as usize])?;
        }
        Ok(())
    }

    /// Whole-page copy onto a RAM page (exchange primitive for COPY/LOAD, §4).
    pub fn copy_page_from(&mut self, dst_ram_page: i8, src: &Page) -> Result<(), MemError> {
        if !Self::is_ram(dst_ram_page) {
            return Err(MemError::ReadOnly { page: dst_ram_page });
        }
        let i = Self::pslot(dst_ram_page).unwrap();
        self.pages[i] = *src;
        Ok(())
    }

    /// Loads a ROM page image (e.g. the firmware "program equipment").
    pub fn load_rom_page(&mut self, rom_page: i8, image: Page) -> Result<(), MemError> {
        if !Self::is_rom(rom_page) {
            return Err(MemError::BadAddress { page: rom_page, addr: 0 });
        }
        let i = Self::pslot(rom_page).unwrap();
        self.pages[i] = image;
        Ok(())
    }
}

// ============================================================================
// Level-2 memory (drum `f`, §4): 3 kinds × 6561 pages (q is an 8-trit
// number, −3280..=3280). Page-granular exchange with level-1 RAM only.
// ============================================================================

pub const L2_KIND_MIN: i8 = -1;
pub const L2_KIND_MAX: i8 = 1;
pub const L2_PAGE_MIN: i32 = -3280;
pub const L2_PAGE_MAX: i32 = 3280;
pub const L2_PAGES_PER_KIND: usize = (L2_PAGE_MAX - L2_PAGE_MIN + 1) as usize; // 6561

/// Level-2 memory: three drum kinds (`f[-1]`, `f[0]`, `f[1]`), flat array
/// indexed by `(kind + 1) * 6561 + (q + 3280)`.
#[derive(Clone, Debug)]
pub struct Memory2 {
    pages: Vec<Page>,
}

impl Memory2 {
    pub fn zeroed() -> Self {
        Memory2 { pages: vec![Page::zeroed(); L2_PAGES_PER_KIND * 3] }
    }

    #[inline(always)]
    fn slot(kind: i8, q: i32) -> Option<usize> {
        if (L2_KIND_MIN..=L2_KIND_MAX).contains(&kind)
            && (L2_PAGE_MIN..=L2_PAGE_MAX).contains(&q)
        {
            Some((kind as i32 + 1) as usize * L2_PAGES_PER_KIND + (q - L2_PAGE_MIN) as usize)
        } else {
            None
        }
    }

    pub fn read_page(&self, kind: i8, q: i32) -> Option<&Page> {
        Self::slot(kind, q).map(|i| &self.pages[i])
    }

    pub fn write_page(&mut self, kind: i8, q: i32, page: Page) -> Option<()> {
        Self::slot(kind, q).map(|i| self.pages[i] = page)
    }
}
