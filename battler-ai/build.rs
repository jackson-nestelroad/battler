fn main() {
    let python_dir = "../battler-ai-gemini-py";
    let python_dir = std::path::Path::new(python_dir);
    let build_dir = python_dir.join("build");
    let script = python_dir.join("main.py");
    let spec = python_dir.join("main.spec");

    println!("cargo:rerun-if-changed={}", script.to_str().unwrap());
    println!("cargo:rerun-if-changed={}", spec.to_str().unwrap());

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);
    let python_build_dir = out_dir.join("battler-ai-gemini-py");

    let venv = std::env::var("GEMINI_PYTHON_VENV").unwrap_or_else(|_| "/".to_string());
    let venv = std::path::Path::new(&venv);
    let mut command = std::process::Command::new(venv.join("bin/pyinstaller"));
    let output = command
        .arg("--noconfirm")
        .arg("--workpath")
        .arg(&build_dir)
        .arg("--distpath")
        .arg(&python_build_dir)
        .arg(spec)
        .output()
        .expect("failed to execute pyinstaller");

    if !output.status.success() {
        eprintln!("pyinstaller failed:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        panic!("pyinstaller failed");
    }

    println!("pyinstaller succeeded");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    let executable_path = python_build_dir.join("main/main");
    println!(
        "cargo::rustc-env=GEMINI_PYTHON_EXECUTABLE={}",
        executable_path.to_str().unwrap()
    )
}
