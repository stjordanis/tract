#[cfg(features = "conform")]
extern crate conform;
extern crate flate2;
extern crate image;
extern crate mio_httpc;
extern crate ndarray;
extern crate tar;
#[allow(unused_imports)]
#[macro_use]
extern crate tract_core;
extern crate tract_tensorflow;

use std::{fs, io, path};

use tract_core::TfdResult;

fn download() {
    use std::sync::{Once, ONCE_INIT};
    static START: Once = ONCE_INIT;

    START.call_once(|| do_download().unwrap());
}

fn do_download() -> TfdResult<()> {
    let dir = inception_v3_2016_08_28();
    let dir_partial = dir.clone().with_extension("partial");
    if fs::metadata(&dir).is_ok() {
        println!("Found inception_v3 model to {:?}", dir);
        return Ok(());
    }
    println!("Downloading inception_v3 model in {:?}", dir);
    if fs::metadata(&dir_partial).is_ok() {
        fs::remove_dir_all(&dir_partial).unwrap();
    }
    fs::create_dir_all(&dir_partial)?;
    let resp = mio_httpc::CallBuilder::get()
        .url("http://storage.googleapis.com/download.tensorflow.org/models/inception_v3_2016_08_28_frozen.pb.tar.gz")
        .unwrap()
        .max_response(200_000_000)
        .timeout_ms(600_000)
        .exec()
        .map_err(|e| format!("http error {:?}", e))?;
    println!("Downloaded inception_v3 model, de-archiving to: {:?}", dir);
    let mut archive = ::tar::Archive::new(::flate2::read::GzDecoder::new(&*resp.1));
    archive.unpack(&dir_partial)?;
    fs::rename(dir_partial, dir)?;
    Ok(())
}

pub fn load_labels() -> Vec<String> {
    use std::io::BufRead;
    io::BufReader::new(fs::File::open(imagenet_slim_labels()).unwrap())
        .lines()
        .collect::<::std::io::Result<Vec<String>>>()
        .unwrap()
}

fn inception_v3_2016_08_28() -> path::PathBuf {
    match ::std::env::var("TRAVIS_BUILD_DIR") {
        Ok(t) => path::Path::new(&t)
            .join("cached")
            .join("inception-v3-2016_08_28"),
        _ => path::PathBuf::from(".inception-v3-2016_08_28"),
    }
}

pub fn inception_v3_2016_08_28_frozen() -> path::PathBuf {
    download();
    inception_v3_2016_08_28().join("inception_v3_2016_08_28_frozen.pb")
}

pub fn imagenet_slim_labels() -> path::PathBuf {
    download();
    inception_v3_2016_08_28().join("imagenet_slim_labels.txt")
}

pub fn load_image<P: AsRef<path::Path>>(p: P) -> ::tract_core::Tensor {
    let image = ::image::open(&p).unwrap().to_rgb();
    let resized = ::image::imageops::resize(&image, 299, 299, ::image::FilterType::Triangle);
    let image: ::tract_core::Tensor =
        ::ndarray::Array4::from_shape_fn((1, 299, 299, 3), |(_, y, x, c)| {
            resized[(x as _, y as _)][c] as f32 / 255.0
        }).into_dyn()
        .into();
    image
}

#[cfg(test)]
mod tests {
    extern crate dinghy_test;
    use self::dinghy_test::test_project_path;
    use super::*;
    use std::path;

    const HOPPER: &str = "grace_hopper.jpg";
    pub fn hopper() -> path::PathBuf {
        test_project_path().join(HOPPER)
    }

    #[test]
    fn grace_hopper_is_a_military_uniform() {
        download();
        let tfd = ::tract_tensorflow::for_path(inception_v3_2016_08_28_frozen()).unwrap();
        let plan = ::tract_core::SimplePlan::new(&tfd).unwrap();
        let input = load_image(hopper());
        let outputs = plan.run(tvec![input]).unwrap();
        let labels = load_labels();
        let label_id = outputs[0]
            .as_f32s()
            .unwrap()
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(0u32.cmp(&1)))
            .unwrap()
            .0;
        let label = &labels[label_id];
        assert_eq!(label, "military uniform");
    }
}