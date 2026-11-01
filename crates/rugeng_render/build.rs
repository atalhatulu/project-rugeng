use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=shaders/quad.vert");
    println!("cargo:rerun-if-changed=shaders/quad.frag");

    // glslc mevcutsa shader'ları otomatik derle
    if let Ok(output) = Command::new("glslc").arg("--version").output() {
        if output.status.success() {
            let _ = Command::new("glslc")
                .args(["shaders/quad.vert", "-o", "shaders/quad.vert.spv"])
                .status();
            let _ = Command::new("glslc")
                .args(["shaders/quad.frag", "-o", "shaders/quad.frag.spv"])
                .status();
        }
    }
}
