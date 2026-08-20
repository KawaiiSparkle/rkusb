//! Decode Rockchip NS OTP / eFuse dumps into named fields.
//!
//! Maps follow Linux `rockchip,*-otp` device-tree cells and U-Boot
//! `CFG_CPUID_OFFSET`. This is the non-secure OTP the USB loader can read
//! (`READ_OTP_CHIP` 0x2C). Secure OTP (RSA key hash, Secure Boot fuse) is
//! usually *not* in this dump.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtpMapKind {
    Auto,
    Rk3588,
    Rk3568,
    Px30,
    Generic,
}

impl OtpMapKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "rk3588" | "rk3588s" | "3588" => Some(Self::Rk3588),
            "rk3576" | "3576" => Some(Self::Rk3588),
            "rk3568" | "rk3566" | "3568" | "3566" => Some(Self::Rk3568),
            "px30" | "rk3308" | "rk3399" | "rk3328" | "rk3288" => Some(Self::Px30),
            "generic" => Some(Self::Generic),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Rk3588 => "RK3588/RK3576 NS OTP",
            Self::Rk3568 => "RK3568/RK3566 NS OTP",
            Self::Px30 => "PX30/RK33xx eFuse/OTP",
            Self::Generic => "generic NS OTP",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FieldKind {
    Hex,
    Ascii,
    CpuId,
    U8,
    Bits { shift: u32, width: u32 },
}

struct OtpField {
    name: &'static str,
    offset: usize,
    len: usize,
    kind: FieldKind,
    note: &'static str,
}

struct OtpMap {
    kind: OtpMapKind,
    fields: &'static [OtpField],
}

const RK3588_FIELDS: &[OtpField] = &[
    OtpField {
        name: "cpu-code",
        offset: 0x02,
        len: 2,
        kind: FieldKind::Ascii,
        note: "SoC product code (factory)",
    },
    OtpField {
        name: "cpuid",
        offset: 0x07,
        len: 0x10,
        kind: FieldKind::CpuId,
        note: "16-byte unique chip ID (same source as /proc/cpuinfo Serial)",
    },
    OtpField {
        name: "cpub0-leakage",
        offset: 0x17,
        len: 1,
        kind: FieldKind::U8,
        note: "big-core cluster 0 leakage bin",
    },
    OtpField {
        name: "cpub1-leakage",
        offset: 0x18,
        len: 1,
        kind: FieldKind::U8,
        note: "big-core cluster 1 leakage bin",
    },
    OtpField {
        name: "cpul-leakage",
        offset: 0x19,
        len: 1,
        kind: FieldKind::U8,
        note: "little-core leakage bin",
    },
    OtpField {
        name: "log-leakage",
        offset: 0x1a,
        len: 1,
        kind: FieldKind::U8,
        note: "logic leakage bin",
    },
    OtpField {
        name: "gpu-leakage",
        offset: 0x1b,
        len: 1,
        kind: FieldKind::U8,
        note: "GPU leakage bin",
    },
    OtpField {
        name: "cpu-version",
        offset: 0x1c,
        len: 1,
        kind: FieldKind::Bits {
            shift: 3,
            width: 3,
        },
        note: "CPU revision (bits 3..5)",
    },
    OtpField {
        name: "npu-leakage",
        offset: 0x28,
        len: 1,
        kind: FieldKind::U8,
        note: "NPU leakage bin",
    },
    OtpField {
        name: "codec-leakage",
        offset: 0x29,
        len: 1,
        kind: FieldKind::U8,
        note: "video codec leakage bin",
    },
];

const RK3568_FIELDS: &[OtpField] = &[
    OtpField {
        name: "chip-magic",
        offset: 0x00,
        len: 4,
        kind: FieldKind::Ascii,
        note: "factory ASCII tag (often 'RK5h' / similar)",
    },
    OtpField {
        name: "cpu-code",
        offset: 0x02,
        len: 2,
        kind: FieldKind::Ascii,
        note: "SoC product code",
    },
    OtpField {
        name: "cpuid",
        offset: 0x0a,
        len: 0x10,
        kind: FieldKind::CpuId,
        note: "16-byte unique chip ID (RK356x offset 0x0a, not 0x07)",
    },
];

const PX30_FIELDS: &[OtpField] = &[
    OtpField {
        name: "cpuid",
        offset: 0x07,
        len: 0x10,
        kind: FieldKind::CpuId,
        note: "16-byte unique chip ID (U-Boot default CFG_CPUID_OFFSET)",
    },
    OtpField {
        name: "cpu-leakage",
        offset: 0x17,
        len: 1,
        kind: FieldKind::U8,
        note: "CPU leakage bin",
    },
    OtpField {
        name: "performance",
        offset: 0x1e,
        len: 1,
        kind: FieldKind::Bits {
            shift: 4,
            width: 3,
        },
        note: "performance / speed bin (bits 4..6)",
    },
];

const GENERIC_FIELDS: &[OtpField] = &[
    OtpField {
        name: "chip-magic",
        offset: 0x00,
        len: 4,
        kind: FieldKind::Ascii,
        note: "first 4 bytes if ASCII",
    },
    OtpField {
        name: "cpu-code",
        offset: 0x02,
        len: 2,
        kind: FieldKind::Hex,
        note: "possible SoC product code",
    },
    OtpField {
        name: "cpuid@0x07",
        offset: 0x07,
        len: 0x10,
        kind: FieldKind::CpuId,
        note: "candidate unique ID (RK3588 / PX30 / RK3399)",
    },
    OtpField {
        name: "cpuid@0x0a",
        offset: 0x0a,
        len: 0x10,
        kind: FieldKind::CpuId,
        note: "candidate unique ID (RK3568)",
    },
];

fn map_for(kind: OtpMapKind) -> OtpMap {
    match kind {
        OtpMapKind::Rk3588 => OtpMap {
            kind,
            fields: RK3588_FIELDS,
        },
        OtpMapKind::Rk3568 => OtpMap {
            kind,
            fields: RK3568_FIELDS,
        },
        OtpMapKind::Px30 => OtpMap {
            kind,
            fields: PX30_FIELDS,
        },
        OtpMapKind::Generic | OtpMapKind::Auto => OtpMap {
            kind: OtpMapKind::Generic,
            fields: GENERIC_FIELDS,
        },
    }
}

/// Guess OTP map from READ_CHIP_INFO's 4-char LE name ("3588", "3568", …).
pub fn detect_otp_map(chip_name: &str) -> OtpMapKind {
    let n = chip_name.to_ascii_uppercase();
    if n.contains("3588") || n.contains("3576") || n.contains("3506") {
        OtpMapKind::Rk3588
    } else if n.contains("3568") || n.contains("3566") || n.contains("3562") {
        OtpMapKind::Rk3568
    } else if n.contains("3399")
        || n.contains("3328")
        || n.contains("3308")
        || n.contains("PX30")
        || n.contains("3288")
        || n.contains("3368")
    {
        OtpMapKind::Px30
    } else {
        OtpMapKind::Generic
    }
}

fn hex_bytes(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn ascii_preview(data: &[u8]) -> Option<String> {
    if data
        .iter()
        .all(|&b| b == 0 || b.is_ascii_graphic() || b == b' ')
        && data.iter().any(|&b| b.is_ascii_graphic())
    {
        Some(
            data.iter()
                .map(|&b| {
                    if b.is_ascii_graphic() || b == b' ' {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect(),
        )
    } else {
        None
    }
}

fn is_blank(data: &[u8]) -> bool {
    data.iter().all(|&b| b == 0 || b == 0xff)
}

fn format_field(data: &[u8], field: &OtpField) -> Option<String> {
    if field.offset + field.len > data.len() {
        return None;
    }
    let slice = &data[field.offset..field.offset + field.len];
    let value = match field.kind {
        FieldKind::Hex | FieldKind::U8 => {
            if is_blank(slice) {
                format!("<empty> ({})", hex_bytes(slice))
            } else {
                hex_bytes(slice)
            }
        }
        FieldKind::Ascii => ascii_preview(slice)
            .map(|s| format!("{s:?}"))
            .unwrap_or_else(|| hex_bytes(slice)),
        FieldKind::CpuId => {
            if is_blank(slice) {
                "<empty / unprogrammed>".to_string()
            } else {
                let hex = hex_bytes(slice);
                match ascii_preview(slice) {
                    Some(s) if s.chars().any(|c| c.is_ascii_alphanumeric()) => {
                        format!("{hex}  ascii={s:?}")
                    }
                    _ => hex,
                }
            }
        }
        FieldKind::Bits { shift, width } => {
            let raw = slice[0] as u32;
            let mask = if width >= 32 {
                u32::MAX
            } else {
                (1u32 << width) - 1
            };
            let v = (raw >> shift) & mask;
            format!("{v} (raw=0x{raw:02x}, bits {shift}..{})", shift + width - 1)
        }
    };
    Some(value)
}

/// Human-readable OTP report. `chip_name` is the READ_CHIP_INFO tag, if known.
pub fn format_otp_report(data: &[u8], chip_name: Option<&str>, map_kind: OtpMapKind) -> String {
    let resolved = if map_kind == OtpMapKind::Auto {
        detect_otp_map(chip_name.unwrap_or(""))
    } else {
        map_kind
    };
    let map = map_for(resolved);
    let mut out = String::new();
    out.push_str("OTP / eFuse decode (non-secure region the USB loader can read)\n");
    if let Some(name) = chip_name {
        out.push_str(&format!("  Chip info: {name}\n"));
    }
    out.push_str(&format!("  Map: {}\n", map.kind.name()));
    out.push_str(&format!("  Dump length: {} bytes\n\n", data.len()));

    out.push_str("Named fields\n");
    out.push_str(&format!(
        "  {:<16} {:>8} {:>6}  {}\n",
        "name", "offset", "len", "value"
    ));
    for field in map.fields {
        match format_field(data, field) {
            Some(value) => {
                out.push_str(&format!(
                    "  {:<16} {:>#8x} {:>6}  {value}\n",
                    field.name, field.offset, field.len
                ));
                if !field.note.is_empty() {
                    out.push_str(&format!("  {:<16}          {}\n", "", field.note));
                }
            }
            None => out.push_str(&format!(
                "  {:<16} {:>#8x} {:>6}  <truncated — dump more bytes>\n",
                field.name, field.offset, field.len
            )),
        }
    }

    let programmed: usize = data.iter().filter(|&&b| b != 0 && b != 0xff).count();
    out.push('\n');
    out.push_str("Security notes\n");
    out.push_str(&format!(
        "  Non-erased bytes: {programmed}/{} ({:.0}%)\n",
        data.len(),
        if data.is_empty() {
            0.0
        } else {
            100.0 * programmed as f64 / data.len() as f64
        }
    ));
    out.push_str(
        "  Secure Boot RSA key hash and the Secure Boot enable fuse live in *secure* OTP\n",
    );
    out.push_str(
        "  (OTP_S). This dump is NS OTP: CPUID, leakage bins, and factory SoC identity.\n",
    );
    out.push_str(
        "  A non-empty CPUID is the chip unique ID. Empty CPUID means the loader did not\n",
    );
    out.push_str("  return that cell (short dump, wrong map, or unfused sample).\n");

    out.push_str("\nAnnotated hex dump\n");
    out.push_str(&annotated_hexdump(data, map.fields));
    out
}

fn field_at(fields: &[OtpField], offset: usize) -> Option<&'static str> {
    fields.iter().find_map(|f| {
        (offset >= f.offset && offset < f.offset + f.len).then_some(f.name)
    })
}

fn annotated_hexdump(data: &[u8], fields: &[OtpField]) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let base = i * 16;
        out.push_str(&format!("{base:08x}  "));
        for (j, b) in chunk.iter().enumerate() {
            out.push_str(&format!("{b:02x} "));
            if j == 7 {
                out.push(' ');
            }
        }
        for j in chunk.len()..16 {
            out.push_str("   ");
            if j == 7 {
                out.push(' ');
            }
        }
        out.push_str(" |");
        for b in chunk {
            let c = if b.is_ascii_graphic() || *b == b' ' {
                *b as char
            } else {
                '.'
            };
            out.push(c);
        }
        out.push('|');
        let mut labels = Vec::new();
        for off in base..base + chunk.len() {
            if let Some(name) = field_at(fields, off) {
                if !labels.contains(&name) {
                    labels.push(name);
                }
            }
        }
        if !labels.is_empty() {
            out.push_str("  ");
            out.push_str(&labels.join(", "));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rk3568_and_parses_odroid_like_dump() {
        let mut data = vec![0u8; 48];
        data[0..16].copy_from_slice(&[
            0x52, 0x4b, 0x35, 0x68, 0x02, 0x00, 0xfe, 0x42, 0x10, 0x01, 0x54, 0x47, 0x4e, 0x4b,
            0x39, 0x30,
        ]);
        assert_eq!(detect_otp_map("3568"), OtpMapKind::Rk3568);
        let report = format_otp_report(&data, Some("3568"), OtpMapKind::Auto);
        assert!(report.contains("cpuid"));
        assert!(report.contains("TGNK90") || report.contains("54474e4b3930"));
        assert!(report.contains("RK5h") || report.contains("chip-magic"));
    }

    #[test]
    fn rk3588_cpuid_offset() {
        let mut data = vec![0u8; 64];
        data[0x02..0x04].copy_from_slice(b"58");
        for (i, b) in (0u8..16).enumerate() {
            data[0x07 + i] = b;
        }
        let report = format_otp_report(&data, Some("3588"), OtpMapKind::Auto);
        assert!(report.contains("000102030405060708090a0b0c0d0e0f"));
        assert!(report.contains("cpu-code"));
        assert!(report.contains("RK3588"));
    }
}
