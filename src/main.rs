#![warn(clippy::pedantic, clippy::nursery, clippy::all)]

use std::{
    collections::HashMap,
    fs, io,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use hyper::{
    body::Bytes,
    service::{make_service_fn, service_fn},
    Body, Error, Response,
};

#[tokio::main]
async fn main() {
    let path = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".to_string()));
    let paths = walk_dir(path.clone()).expect("Failure reading directories");
    let files = Arc::new(read_files(paths, &path).expect("Failure reading files!"));

    let svc = make_service_fn(move |_| {
        let state = files.clone();
        async move {
            Ok::<_, Error>(service_fn(move |req| {
                let state = state.clone();
                async move {
                    let mut path = req.uri().path().to_string();
                    if path.ends_with('/') {
                        path += "index.html";
                    }
                    let data = state.get(&path).cloned().unwrap_or_default();
                    let resp = if data.is_empty() {
                        Response::builder()
                            .status(404)
                            .body(Body::from("404 not found"))
                            .unwrap()
                    } else {
                        Response::builder()
                            .status(200)
                            .body(Body::from(data))
                            .unwrap()
                    };
                    Ok::<_, Error>(resp)
                }
            }))
        }
    });

    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();
    let server = hyper::Server::bind(&addr).serve(svc);
    println!("[INFO] Listening on http://{}", &addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

fn walk_dir(path: PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();
    for item_result in fs::read_dir(path).expect("Failed to read target directory!") {
        let item = item_result?;
        let filetype = item.file_type()?;
        if filetype.is_dir() || filetype.is_symlink() {
            files.append(&mut walk_dir(item.path())?);
        } else if filetype.is_file() {
            files.push(item.path());
        } else {
            println!("Item is not file, directory, or symlink!");
        }
    }
    Ok(files)
}

fn read_files(paths: Vec<PathBuf>, path: &Path) -> io::Result<HashMap<String, Bytes>> {
    let mut items: HashMap<String, Bytes> = HashMap::new();
    for filepath in paths {
        let name = clean_name(
            filepath.to_str().unwrap().to_string(),
            path.to_str().unwrap(),
        );
        items.insert(name, Bytes::from(fs::read(filepath)?));
    }
    Ok(items)
}

fn clean_name(mut name: String, path: &str) -> String {
    if name.starts_with("./") {
        name = name.replace("./", "");
    }
    if name.starts_with(path) {
        name = name.replace(path, "");
    }
    if !name.starts_with('/') {
        name = format!("/{}", name);
    }
    name
}
