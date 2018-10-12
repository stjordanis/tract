extern crate git2;
extern crate tempdir;

use std::{fs, path};

use tempdir::TempDir;

pub fn dir() -> path::PathBuf {
    match ::std::env::var("TRAVIS_BUILD_DIR") {
        Ok(t) => path::Path::new(&t).join("cached").join("onnx-checkout"),
        _ => ".onnx".into(),
    }
}

pub fn ensure_onnx_git_checkout() {
    use std::sync::Once;
    static START: Once = Once::new();
    START.call_once(|| {
        if !dir().exists() {
            let _ = fs::create_dir_all(dir().parent().unwrap());
            let tmp = TempDir::new("onnx").unwrap();
            let url = "https://github.com/onnx/onnx";
            ::git2::Repository::clone(url, &tmp).unwrap();
            fs::rename(tmp.into_path(), dir()).unwrap();
        }
    });
}

pub fn make_test_file(tests_set: &str) {
    use std::io::Write;
    ensure_onnx_git_checkout();
    let node_tests = dir().join("onnx/backend/test/data").join(tests_set);
    assert!(node_tests.exists());
    let working_list_file = path::PathBuf::from("tests")
        .join(tests_set)
        .with_extension("txt");
    let working_list: Vec<String> = if let Ok(list) = fs::read_to_string(&working_list_file) {
        list.split("\n").map(|s| s.to_string()).collect()
    } else {
        vec![]
    };
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = path::PathBuf::from(out_dir);
    let test_dir = out_dir.join("tests");
    fs::create_dir_all(&test_dir).unwrap();
    let test_file = test_dir.join(tests_set).with_extension("rs");
    let mut rs = fs::File::create(test_file).unwrap();
    let mut tests: Vec<String> = fs::read_dir(&node_tests)
        .unwrap()
        .map(|de| de.unwrap().file_name().to_str().unwrap().to_owned())
        .collect();
    tests.sort();
    writeln!(rs, "mod {} {{", tests_set.replace("-","_"));
    for (s, optim) in &[("plain", false), ("optim", true)] {
        writeln!(rs, "mod {} {{", s);
        for t in &tests {
            writeln!(rs, "#[test]");
            if !working_list.contains(&t) {
                writeln!(rs, "#[ignore]");
            }
            writeln!(rs, "fn {}() {{", t);
            writeln!(rs, "::onnx::run_one({:?}, {:?}, {:?})", node_tests, t, optim);
            writeln!(rs, "}}");
        }
        writeln!(rs, "}}");
    }
    writeln!(rs, "}}");
}

fn main() {
    ensure_onnx_git_checkout();
    make_test_file("node");
    make_test_file("real");
    make_test_file("simple");
    make_test_file("pytorch-operator");
    make_test_file("pytorch-converted");
}