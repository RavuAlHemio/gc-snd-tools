use std::io::{self, Cursor, Write};

use goblin::container::Ctx;
use scroll::IOwrite;


pub trait ElfWriteExt {
    fn size() -> usize;
    fn write_elf<W: Write>(&self, target: &mut W, ctx: Ctx) -> Result<(), io::Error>;
}
impl ElfWriteExt for goblin::elf32::header::Header {
    fn size() -> usize {
        goblin::elf32::header::header32::SIZEOF_EHDR
    }

    fn write_elf<W: Write>(&self, target: &mut W, ctx: Ctx) -> Result<(), io::Error> {
        target.write_all(&self.e_ident)?;
        target.iowrite_with(self.e_type, ctx.le)?;
        target.iowrite_with(self.e_machine, ctx.le)?;
        target.iowrite_with(self.e_version, ctx.le)?;
        target.iowrite_with(self.e_entry, ctx.le)?;
        target.iowrite_with(self.e_phoff, ctx.le)?;
        target.iowrite_with(self.e_shoff, ctx.le)?;
        target.iowrite_with(self.e_flags, ctx.le)?;
        target.iowrite_with(self.e_ehsize, ctx.le)?;
        target.iowrite_with(self.e_phentsize, ctx.le)?;
        target.iowrite_with(self.e_phnum, ctx.le)?;
        target.iowrite_with(self.e_shentsize, ctx.le)?;
        target.iowrite_with(self.e_shnum, ctx.le)?;
        target.iowrite_with(self.e_shstrndx, ctx.le)?;
        Ok(())
    }
}
impl ElfWriteExt for goblin::elf32::section_header::SectionHeader {
    fn size() -> usize {
        goblin::elf32::section_header::section_header32::SIZEOF_SHDR
    }

    fn write_elf<W: Write>(&self, target: &mut W, ctx: Ctx) -> Result<(), io::Error> {
        target.iowrite_with(self.sh_name, ctx.le)?;
        target.iowrite_with(self.sh_type, ctx.le)?;
        target.iowrite_with(self.sh_flags, ctx.le)?;
        target.iowrite_with(self.sh_addr, ctx.le)?;
        target.iowrite_with(self.sh_offset, ctx.le)?;
        target.iowrite_with(self.sh_size, ctx.le)?;
        target.iowrite_with(self.sh_link, ctx.le)?;
        target.iowrite_with(self.sh_info, ctx.le)?;
        target.iowrite_with(self.sh_addralign, ctx.le)?;
        target.iowrite_with(self.sh_entsize, ctx.le)?;
        Ok(())
    }
}
impl ElfWriteExt for goblin::elf32::sym::Sym {
    fn size() -> usize {
        goblin::elf32::sym::sym32::SIZEOF_SYM
    }

    fn write_elf<W: Write>(&self, target: &mut W, ctx: Ctx) -> Result<(), io::Error> {
        target.iowrite_with(self.st_name, ctx.le)?;
        target.iowrite_with(self.st_value, ctx.le)?;
        target.iowrite_with(self.st_size, ctx.le)?;
        target.iowrite_with(self.st_info, ctx.le)?;
        target.iowrite_with(self.st_other, ctx.le)?;
        target.iowrite_with(self.st_shndx, ctx.le)?;
        Ok(())
    }
}

pub trait PadExt {
    fn pad_to(&mut self, multiple_of: u64) -> Result<(), io::Error>;
}
impl PadExt for Cursor<&mut Vec<u8>> {
    fn pad_to(&mut self, multiple_of: u64) -> Result<(), io::Error> {
        while self.position() % multiple_of != 0 {
            self.write_all(&[0x00])?;
        }
        Ok(())
    }
}
