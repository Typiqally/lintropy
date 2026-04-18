//! Pack the VS Code / Cursor query-highlight extension into a `.vsix`
//! next to the compiled binary so the CLI can embed it via `include_bytes!`.
//!
//! No `vsce` / node dep: a `.vsix` is just a zip with a small manifest.
//! VS Code's `--install-extension` only needs `[Content_Types].xml`,
//! `extension.vsixmanifest`, and `extension/<files>`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

const EXT_SRC: &str = "editors/vscode/lintropy-query-syntax";
const EXT_FILES: &[&str] = &[
    "package.json",
    "language-configuration.json",
    "LICENSE.txt",
    "syntaxes/lintropy-query.tmLanguage.json",
    "syntaxes/lintropy-query-yaml.injection.tmLanguage.json",
];

fn main() {
    let manifest_dir = PathBuf::from(env_must("CARGO_MANIFEST_DIR"));
    let ext_src = manifest_dir.join(EXT_SRC);
    let out_dir = PathBuf::from(env_must("OUT_DIR"));
    let vsix_path = out_dir.join("lintropy-query-syntax.vsix");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", ext_src.display());

    let pkg_json: serde_json::Value =
        serde_json::from_slice(&fs::read(ext_src.join("package.json")).expect("read package.json"))
            .expect("parse package.json");
    let id = pkg_json["name"].as_str().expect("name");
    let version = pkg_json["version"].as_str().expect("version");
    let publisher = pkg_json["publisher"].as_str().expect("publisher");
    let display_name = pkg_json["displayName"].as_str().expect("displayName");
    let description = pkg_json["description"].as_str().expect("description");

    let file = fs::File::create(&vsix_path).expect("create vsix");
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip_str(&mut zip, "[Content_Types].xml", CONTENT_TYPES, opts);
    zip_str(
        &mut zip,
        "extension.vsixmanifest",
        &manifest(id, version, publisher, display_name, description),
        opts,
    );
    for rel in EXT_FILES {
        let src = ext_src.join(rel);
        println!("cargo:rerun-if-changed={}", src.display());
        zip_file(&mut zip, &format!("extension/{rel}"), &src, opts);
    }
    zip.finish().expect("finish vsix");
}

fn env_must(var: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| panic!("{var} must be set by cargo"))
}

fn zip_str(zip: &mut ZipWriter<fs::File>, name: &str, body: &str, opts: SimpleFileOptions) {
    zip.start_file(name, opts).expect("start_file");
    zip.write_all(body.as_bytes()).expect("write");
}

fn zip_file(zip: &mut ZipWriter<fs::File>, name: &str, src: &Path, opts: SimpleFileOptions) {
    let bytes = fs::read(src).unwrap_or_else(|err| panic!("read {}: {err}", src.display()));
    zip.start_file(name, opts).expect("start_file");
    zip.write_all(&bytes).expect("write");
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="json" ContentType="application/json"/>
  <Default Extension="txt" ContentType="text/plain"/>
  <Default Extension="vsixmanifest" ContentType="text/xml"/>
</Types>
"#;

fn manifest(
    id: &str,
    version: &str,
    publisher: &str,
    display_name: &str,
    description: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011" xmlns:d="http://schemas.microsoft.com/developer/vsx-schema-design/2011">
  <Metadata>
    <Identity Language="en-US" Id="{id}" Version="{version}" Publisher="{publisher}"/>
    <DisplayName>{display}</DisplayName>
    <Description xml:space="preserve">{description}</Description>
    <Categories>Programming Languages</Categories>
    <GalleryFlags>Public</GalleryFlags>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/>
  </Assets>
</PackageManifest>
"#,
        id = xml_escape(id),
        version = xml_escape(version),
        publisher = xml_escape(publisher),
        display = xml_escape(display_name),
        description = xml_escape(description),
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
