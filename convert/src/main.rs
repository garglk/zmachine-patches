use std::fs;

use clap::Parser;
use itertools::Itertools;
use serde::{Deserialize, Deserializer};
use serde_derive::Deserialize;

#[derive(Debug, Clone, clap::ValueEnum)]
enum Mode {
    BocfelRuntime,
    BocfelCompiletime,
}

#[derive(Debug, Parser)]
struct Options {
    #[clap(short, long)]
    mode: Mode,
}

fn deserialize_serial<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.len() != 6 {
        return Err(serde::de::Error::custom("Serial number must be 6 characters"));
    }
    Ok(s)
}

#[derive(Debug, Deserialize)]
struct Replacement {
    addr: u32,
    r#in: Vec<u8>,
    out: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct Patch {
    title: String,
    #[serde(deserialize_with = "deserialize_serial")]
    serial: String,
    release: u16,
    checksum: u16,
    replacements: Vec<Replacement>,
}

fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    let patches = fs::read_to_string("../patches.json")?;
    let patches = json_comments::StripComments::new(patches.as_bytes());
    let patches: Vec<Patch> = serde_json::from_reader(patches)?;

    validate(&patches)?;

    match options.mode {
        Mode::BocfelRuntime => generate_bocfel_runtime(&patches),
        Mode::BocfelCompiletime => generate_bocfel_compiletime(&patches),
    }

    Ok(())
}

fn validate(patches: &[Patch]) -> anyhow::Result<()> {
    for patch in patches {
        for replacement in &patch.replacements {
            if replacement.r#in.len() != replacement.out.len() {
                anyhow::bail!("replacement at addr {} for {} has length mismatch", replacement.addr, patch.title);
            }
        }
    }

    Ok(())
}

fn generate_bocfel_runtime(patches: &[Patch]) {
    for patch in patches {
        let ifid = if patch.serial.starts_with('8') {
            format!("{}-{}", patch.release, patch.serial)
        } else {
            format!("{}-{}-{:x}", patch.release, patch.serial, patch.checksum)
        };

        println!("# {}\n[{}]", patch.title, ifid);

        for replacement in &patch.replacements {
            println!("0x{:x} {} [{}] [{}]", replacement.addr, replacement.r#in.len(), format_bytes_runtime(&replacement.r#in), format_bytes_runtime(&replacement.out));
        }

        println!()
    }
}

fn format_bytes_runtime(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|byte| format!("{:02x}", byte))
        .join(" ")
}

fn generate_bocfel_compiletime(patches: &[Patch]) {
    for patch in patches {
        println!("{{");
        println!("    \"{}\", \"{}\", {}, 0x{:x},", patch.title, patch.serial, patch.release, patch.checksum);
        println!("    {{");
        for replacement in &patch.replacements {
            println!("        {{");
            println!("            0x{:x}, {},", replacement.addr, replacement.r#in.len());
            println!("            {{{}}},", format_bytes_compiletime(&replacement.r#in));
            println!("            {{{}}},", format_bytes_compiletime(&replacement.out));
            println!("        }},");
        }
        println!("    }},");
        println!("}},");
    }
}

fn format_bytes_compiletime(bytes: &[u8]) -> String{
    bytes.iter()
        .map(|byte| format!("0x{:02x}", byte))
        .join(", ")
}
