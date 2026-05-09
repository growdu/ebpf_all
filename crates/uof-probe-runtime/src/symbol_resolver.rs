//! Symbol resolver module - resolves function symbols to memory addresses

use std::process::Command;
use anyhow::{Result, Context};

pub struct SymbolResolver;

impl SymbolResolver {
    pub fn new() -> Self { Self }

    pub fn resolve(&self, _pid: u32, binary_path: &str, symbol: &str) -> Result<u64> {
        let output = Command::new("nm")
            .args(["-D", binary_path])
            .output()
            .context("Failed to run nm")?;

        if !output.status.success() {
            anyhow::bail!("nm failed for {}", binary_path);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let addr_str = parts[0];
                let sym = parts[2];
                if sym == symbol {
                    let addr = u64::from_str_radix(addr_str, 16)
                        .context("invalid address")?;
                    return Ok(addr);
                }
            }
        }
        anyhow::bail!("Symbol {} not found in {}", symbol, binary_path)
    }

    pub fn list_symbols(&self, binary_path: &str) -> Result<Vec<String>> {
        let output = Command::new("nm")
            .args(["-D", binary_path])
            .output()
            .context("Failed to run nm")?;

        if !output.status.success() {
            anyhow::bail!("nm failed for {}", binary_path);
        }

        let mut symbols = Vec::new();
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let sym = parts[2];
                if !sym.starts_with('_') && !sym.contains('@') && sym.len() > 2 {
                    symbols.push(sym.to_string());
                }
            }
        }

        symbols.sort();
        symbols.dedup();
        Ok(symbols)
    }

    pub fn resolve_addr(&self, binary_path: &str, addr: u64) -> Result<String> {
        let output = Command::new("addr2line")
            .args(["-e", binary_path, "-C", "-f", &format!("0x{:x}", addr)])
            .output()
            .context("Failed to run addr2line")?;

        if !output.status.success() {
            anyhow::bail!("addr2line failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()
            .unwrap_or("unknown")
            .to_string();

        if first_line == "??" || first_line.is_empty() {
            Ok("unknown".to_string())
        } else {
            Ok(first_line)
        }
    }
}

impl Default for SymbolResolver {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_list_symbols() {
        let resolver = SymbolResolver::new();
        let symbols = resolver.list_symbols("/bin/ls").unwrap();
        assert!(!symbols.is_empty());
    }
}