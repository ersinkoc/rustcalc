fn main() {
    #[cfg(windows)]
    {
        use std::{env, fs, path::PathBuf};
        println!("cargo:rerun-if-changed=assets/rustcalc.ico");
        let icon =
            PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets/rustcalc.ico");
        let icon = icon
            .to_str()
            .expect("UTF-8 resource path")
            .replace('\\', "/");
        let version = env!("CARGO_PKG_VERSION");
        let numbers = version.replace('.', ",");
        let rc = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("rustcalc.rc");
        fs::write(
            &rc,
            format!(
                r#"
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numbers},0
PRODUCTVERSION {numbers},0
FILEFLAGSMASK 0x3f
FILEOS 0x40004
FILETYPE 1
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "FileDescription", "RustCalc"
            VALUE "FileVersion", "{version}"
            VALUE "OriginalFilename", "rustcalc.exe"
            VALUE "ProductName", "RustCalc"
            VALUE "ProductVersion", "{version}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
            ),
        )
        .expect("write Windows resources");
        embed_resource::compile_for(&rc, ["rustcalc"], embed_resource::NONE)
            .manifest_optional()
            .expect("embed Windows resources");
    }
}
