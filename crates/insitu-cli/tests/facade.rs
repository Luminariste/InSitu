//! Les commandes `insitu facade …`, lancées comme le ferait un utilisateur.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("insitu-cli-facade-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn insitu(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_insitu"))
        .args(args)
        .output()
        .expect("lancement de insitu")
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "échec : {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_plan_is_inspected_imported_scaled_and_rendered() {
    let temp = TempDir::new("svg");
    let plan = temp.0.join("facade.svg");
    fs::write(
        &plan,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="800">
             <rect x="100" y="100" width="1000" height="600" fill="none" stroke="black"/>
           </svg>"#,
    )
    .unwrap();
    let plan = plan.to_str().unwrap();
    let project = temp.0.join("Essai.insitu");
    let project = project.to_str().unwrap();

    let info: Value = serde_json::from_str(&ok(&insitu(&["facade", "inspect", plan]))).unwrap();
    assert_eq!(info["kind"], "svg");
    assert_eq!(info["width"], 1200.0);

    ok(&insitu(&["project", "new", project]));
    let facade: Value =
        serde_json::from_str(&ok(&insitu(&["facade", "import", project, plan]))).unwrap();
    assert_eq!(facade["source"], "facade/facade.svg");

    // Le rectangle mesure 1 000 unités : on lui donne 20 m.
    let text = ok(&insitu(&[
        "facade", "scale", project, "100", "100", "1100", "100", "20000",
    ]));
    assert!(text.contains("Façade : 24.000 m × 16.000 m"), "{text}");

    let out = temp.0.join("rendu.png");
    ok(&insitu(&[
        "facade",
        "render",
        plan,
        out.to_str().unwrap(),
        "--width",
        "600",
    ]));
    let (width, height) = image_size(&out);
    assert_eq!((width, height), (600, 400));
}

#[test]
fn a_wrong_plan_gives_a_clear_message() {
    let output = insitu(&["facade", "inspect", "plan.dwg"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("DXF"));
    let output = insitu(&["facade", "inspect"]);
    assert_eq!(output.status.code(), Some(2));
}

/// Largeur et hauteur d'un PNG, lues dans son en-tête.
fn image_size(path: &std::path::Path) -> (u32, u32) {
    let bytes = fs::read(path).unwrap();
    let read = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
    (read(16), read(20))
}
