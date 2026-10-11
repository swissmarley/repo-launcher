//! Pinned runtime versions and their SHA-256 hashes (see DECISIONS D-015). Hashes come from
//! each project's published checksum file; bump them together with the version.

pub const UV_VERSION: &str = "0.13.0";
pub const BUN_VERSION: &str = "1.4.3";

/// Supported Node LTS lines, newest first. Projects asking for another version get the
/// nearest pinned line.
pub const NODE_LINES: &[(u32, &str)] = &[(24, "24.21.0"), (22, "22.23.3")];
pub const NODE_DEFAULT_MAJOR: u32 = 24;

/// Default package-manager versions when a project doesn't name one.
pub const PNPM_DEFAULT: &str = "10";
pub const YARN_CLASSIC_DEFAULT: &str = "1.22.22";
pub const YARN_BERRY_DEFAULT: &str = "4";

pub struct Asset {
    pub file: String,
    pub sha256: &'static str,
}

fn os_arch() -> (&'static str, &'static str) {
    (std::env::consts::OS, std::env::consts::ARCH)
}

pub fn uv() -> Option<Asset> {
    let (file, sha256) = match os_arch() {
        ("macos", "aarch64") => (
            "uv-aarch64-apple-darwin.tar.gz",
            "a9c1b29002cf3c83f07fa9cd8a887a3be0107d90e23189721221e7257db8e3d6",
        ),
        ("macos", "x86_64") => (
            "uv-x86_64-apple-darwin.tar.gz",
            "5f44dcbde809b632f47c36fadb241cb4d6f9af71d0c8f172b5d2026d3dde742c",
        ),
        ("windows", "x86_64") => (
            "uv-x86_64-pc-windows-msvc.zip",
            "088962f9e7b7bd9ea740c04c650b2a21c8928c345bd99ac24350dc924dba656c",
        ),
        ("windows", "aarch64") => (
            "uv-aarch64-pc-windows-msvc.zip",
            "cb54028b59aa87f11cf6dec293ce000420043a49a8aea522e822a671321f8932",
        ),
        ("linux", "x86_64") => (
            "uv-x86_64-unknown-linux-gnu.tar.gz",
            "1468ebd5a5541121837c5a2817b9972ba6090fa6caa3d142620850a47fb75154",
        ),
        ("linux", "aarch64") => (
            "uv-aarch64-unknown-linux-gnu.tar.gz",
            "3ccfb6af6e242433eb552f7d9676abd5412c6595c497e990d9c8cb7b5bd4d2c3",
        ),
        _ => return None,
    };
    Some(Asset {
        file: file.into(),
        sha256,
    })
}

pub fn node(version: &str) -> Option<Asset> {
    let (platform, ext) = match os_arch() {
        ("macos", "aarch64") => ("darwin-arm64", "tar.gz"),
        ("macos", "x86_64") => ("darwin-x64", "tar.gz"),
        ("linux", "x86_64") => ("linux-x64", "tar.gz"),
        ("linux", "aarch64") => ("linux-arm64", "tar.gz"),
        ("windows", "x86_64") => ("win-x64", "zip"),
        ("windows", "aarch64") => ("win-arm64", "zip"),
        _ => return None,
    };
    let sha256 = match (version, platform) {
        ("24.21.0", "darwin-arm64") => {
            "bed7eea5325e1108f32ce5228ddd6a5f0f08a499ee42aa7442aea583702f6057"
        }
        ("24.21.0", "darwin-x64") => {
            "1462cb3b3046b815cf8ea436d3da450ec1a9f11dac7e5a46b0ada5305d7e8097"
        }
        ("24.21.0", "linux-arm64") => {
            "724282c3b43aec998aa9527380465b45d229e021b58035f5f4f63095eabfe5d5"
        }
        ("24.21.0", "linux-x64") => {
            "6e1db87ef58b8819e5d5402eff1536491b18edd8eb7bee5ef7897876e88dc5ff"
        }
        ("24.21.0", "win-arm64") => {
            "8779b1bde1d39f8d420e3b57aa657b39891af434d3de44a919044cec06785921"
        }
        ("24.21.0", "win-x64") => {
            "158f7685b44de51f6c0df1d153526cbcd3e1bc739a8dfc607721cef75de9e541"
        }
        ("22.23.3", "darwin-arm64") => {
            "23b25245dcfb9af7262f8ff142e9e2e0af025368117329e7a7458a51e5922f53"
        }
        ("22.23.3", "darwin-x64") => {
            "8a677b0219178efd6eb0e475457c4afb452b521a92f6e67845a73bd85727f2a8"
        }
        ("22.23.3", "linux-arm64") => {
            "5ced2d48d1d7198739b7f86804de0171aefb6823b684b12341d3321afc3cb0b2"
        }
        ("22.23.3", "linux-x64") => {
            "1084aa36196bba4c3a5e69a1ee388a6e4ff729dad09445fbcd434b28fe3c24af"
        }
        ("22.23.3", "win-arm64") => {
            "33dad22e4cef5ee8f9fbb1b0d037fdacd0e56d12a4580f0d63f68b894deab535"
        }
        ("22.23.3", "win-x64") => {
            "2b0ff57b049cda1bbcea2240eec20467018713c1efe1f7360c2681859b90ed71"
        }
        _ => return None,
    };
    Some(Asset {
        file: format!("node-v{version}-{platform}.{ext}"),
        sha256,
    })
}

pub fn bun() -> Option<Asset> {
    let (file, sha256) = match os_arch() {
        ("macos", "aarch64") => (
            "bun-darwin-aarch64.zip",
            "80afd4c069b46a6fa8f3ec520bebc750a7843e4a8d5b9583d29ad436c00a3403",
        ),
        ("macos", "x86_64") => (
            "bun-darwin-x64.zip",
            "62660e6438eb872278877acca77e1152d5d031324df368d4a81b428211489d2d",
        ),
        ("linux", "aarch64") => (
            "bun-linux-aarch64.zip",
            "efa9813da5ed72423bf847f916e8d2c47c0d776add972354026a75e10da9aa21",
        ),
        ("linux", "x86_64") => (
            "bun-linux-x64.zip",
            "3e730528d89775f03f87f05c06787c4df4099d0a37f3f252e7b829cb23816e18",
        ),
        ("windows", "aarch64") => (
            "bun-windows-aarch64.zip",
            "5e8f142ef722cbc9f67dc068a166e46ee72be8307b38cac7186589d2face668f",
        ),
        ("windows", "x86_64") => (
            "bun-windows-x64.zip",
            "0bf84e808552beea67bf1202a287a35c6c825d837b287b3f8e9dd513dba35d3d",
        ),
        _ => return None,
    };
    Some(Asset {
        file: file.into(),
        sha256,
    })
}

/// The pinned Node version for a requested major line (nearest supported line otherwise).
pub fn node_version_for(major: Option<u32>) -> &'static str {
    let want = major.unwrap_or(NODE_DEFAULT_MAJOR);
    NODE_LINES
        .iter()
        .find(|(m, _)| *m == want)
        .or_else(|| {
            NODE_LINES
                .iter()
                .filter(|(m, _)| *m <= want)
                .max_by_key(|(m, _)| *m)
        })
        .or_else(|| NODE_LINES.last())
        .map(|(_, v)| *v)
        .unwrap_or("24.21.0")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_lines() {
        assert_eq!(node_version_for(None), "24.21.0");
        assert_eq!(node_version_for(Some(22)), "22.23.3");
        assert_eq!(node_version_for(Some(23)), "22.23.3");
        assert_eq!(node_version_for(Some(26)), "24.21.0");
        assert_eq!(node_version_for(Some(18)), "22.23.3");
    }

    #[test]
    fn every_platform_we_ship_has_pins() {
        for (major, version) in NODE_LINES {
            assert!(
                node(version).is_some(),
                "node {major} unpinned on this platform"
            );
        }
        assert!(uv().is_some());
        assert!(bun().is_some());
    }
}
