use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

pub struct MmapReader {
    mmap: Mmap,
}

impl MmapReader {
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open file: {:?}", path))?;
        
        let metadata = file.metadata()?;
        if metadata.len() == 0 {
            return Err(anyhow::anyhow!("File is empty"));
        }
        
        // Safety: We're only reading the file and not modifying it
        let mmap = unsafe { 
            let mmap = Mmap::map(&file)?;
            // Advise the kernel about our access pattern
            mmap.advise(memmap2::Advice::Sequential)?;
            mmap
        };
        
        Ok(Self { mmap })
    }
    
    pub fn lines(&self) -> MmapLines<'_> {
        MmapLines {
            data: &self.mmap[..],
            position: 0,
        }
    }
}

pub struct MmapLines<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Iterator for MmapLines<'a> {
    type Item = &'a str;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.data.len() {
            return None;
        }
        
        let remaining = &self.data[self.position..];
        let line_end = remaining
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(remaining.len());
        
        let line_data = &remaining[..line_end];
        self.position += line_end + 1; // +1 to skip the newline
        
        // Convert to UTF-8 string
        std::str::from_utf8(line_data).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_mmap_reader() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "line 1")?;
        writeln!(temp_file, "line 2")?;
        writeln!(temp_file, "line 3")?;
        temp_file.flush()?;
        
        let reader = MmapReader::new(temp_file.path())?;
        let lines: Vec<&str> = reader.lines().collect();
        
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[1], "line 2");
        assert_eq!(lines[2], "line 3");
        
        Ok(())
    }
    
    #[test]
    fn test_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = MmapReader::new(temp_file.path());
        assert!(result.is_err());
    }
}