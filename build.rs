//! 把编译目标三元组（如 `aarch64-unknown-linux-musl`）烘焙进二进制，
//! 在线更新据此在发布清单里挑选对应的安装包。

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=BUILD_TARGET={target}");
}
