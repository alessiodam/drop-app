use ciborium::from_reader;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::{self, BufReader, Error, Seek, Write},
    path::Path,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct ManifestChunk {
    uuid: String,
    index: i64,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct ManifestRecord {
    chunks: Vec<ManifestChunk>,
    permissions: u32,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct Manifest {
    record: HashMap<String, ManifestRecord>,
}

pub async fn unpack() -> Result<(), Error> {
    let chunk_size: u64 = 1024 * 1024 * 16;

    let input = Path::new("/tmp/droplet-dev-output");
    let output = Path::new("/tmp/droplet-dev-rebuilt");

    let manifest_path = input.join("manifest.drop");
    let manifest_file_handle = File::open(manifest_path).unwrap();

    let manifest: Manifest = from_reader(manifest_file_handle).unwrap();
    manifest.record.into_par_iter().for_each(|(key, value)| {
        let file = output.join(key.clone());
        create_dir_all(file.parent().unwrap()).unwrap();
        let mut file_handle = File::create(file).unwrap();

        #[cfg(unix)]
        {
            let mut file_permissions = file_handle.metadata().unwrap().permissions();
            file_permissions.set_mode(value.permissions);
            file_handle.set_permissions(file_permissions).unwrap();
        }

        for chunk in value.chunks {
            let chunk_path = input.join(chunk.uuid + ".bin");
            let chunk_handle = File::open(chunk_path).unwrap();

            let mut chunk_reader = BufReader::new(chunk_handle);

            let offset = u64::try_from(chunk.index).unwrap() * chunk_size;
            file_handle.seek(io::SeekFrom::Start(offset)).unwrap();

            io::copy(&mut chunk_reader, &mut file_handle).unwrap();
            file_handle.flush().unwrap();
        }
    });

    Ok(())
}
