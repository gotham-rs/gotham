use std::{collections::HashMap, fs::File, io::{BufWriter, Write}, net::{SocketAddr, ToSocketAddrs}, sync::atomic::{AtomicU64, Ordering::Relaxed}, time::{Duration, SystemTime}};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use futures_util::future;
use gotham::{
    bind_server, handler::FileOptions, router::{
        build_simple_router,
        builder::{DefineSingleRoute, DrawRoutes},
    }
};
use tempfile::TempDir;
use tokio::{net::TcpListener, runtime::{self, Runtime}};

struct BenchServer {
    runtime: Runtime,
    addr: SocketAddr,
    #[allow(dead_code)]
    tmp: TempDir,
    // sizes of test files
    sizes: Vec<u64>,
    buf_paths: HashMap<String, Option<usize>>,
}

impl BenchServer {
    fn new() -> anyhow::Result<Self> {
        let tmp = TempDir::new()?;
        // temporary datafiles
        let sizes = [10, 17, 24 ].iter().filter_map(|sz| {
            let size = 1 << sz;
            mk_tmp(&tmp, size).ok()
        }).collect();
        let buf_paths = HashMap::from([
            ("default".to_string(), None),
            ("128k".to_string(), Some(1 << 17))
        ]);
        
        let router = build_simple_router(|route| {
            for (path, sz) in &buf_paths {
                let mut opts = FileOptions::from(tmp.path().to_owned());
                if let Some(size) = sz {
                    opts.with_buffer_size(*size);
                }
                route.get(format!("/{path}/*").as_str()).to_dir(opts.to_owned())
            }
        });
        let runtime = runtime::Builder::new_multi_thread()
            .worker_threads(num_cpus::get())
            .thread_name("file_handler-bench")
            .enable_all()
            .build()
            .unwrap();
        // build server manually so that we can capture the actual port instead of 0
        let addr: std::net::SocketAddr = "127.0.0.1:0".to_socket_addrs().unwrap().next().unwrap();
        let listener = runtime.block_on(TcpListener::bind(addr)).unwrap();
        // use any free port
        let addr = listener.local_addr().unwrap();
        let _ = runtime.spawn( async move {
            bind_server(listener, router, future::ok).await;
        });
        std::thread::sleep(Duration::from_millis(100));
        Ok(Self { runtime, addr, tmp, sizes, buf_paths })
    }
}

fn mk_tmp(tmp: &TempDir, size: u64) -> anyhow::Result<u64> {
    let filename = tmp.path().join(format!("{size}"));
    let file = File::create(filename)?;
    let mut w = BufWriter::with_capacity(2 << 16, file);
    // pseudo random data: time stamp as bytes
    let ts_data = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos().to_le_bytes();
    for _ in (0..size).step_by(ts_data.len()) {
        w.write_all(&ts_data)?;
    }
    Ok(size)
}

pub fn filehandler_benchmark(c: &mut Criterion) {
    let server = BenchServer::new().unwrap();

    let runtime = server.runtime;
    let client = reqwest::Client::builder().build().unwrap();
    let counter = AtomicU64::new(0);
    let failed = AtomicU64::new(0);

    for file_size in server.sizes {
        let mut group = c.benchmark_group("server_bench");
        group.throughput(Throughput::Bytes(file_size));
        for (path, buf_size) in &server.buf_paths {
            let url = format!("http://{}/{path}/{file_size}", server.addr);
            let req = client.get(url).build().unwrap();
            group.bench_with_input(BenchmarkId::new("test_file_handler", 
                format!("filesize: {file_size}, bufsize: {buf_size:?}")), &req, |b, req| {
                b.to_async(&runtime).iter(|| async {
                        let r = client.execute(req.try_clone().unwrap()).await;
                        counter.fetch_add(1, Relaxed);
                        match r {
                            Err(_) => { failed.fetch_add(1, Relaxed); },
                            Ok(res) => {
                                // sanity check: did we get what was expected?
                                assert_eq!(res.content_length().unwrap(), file_size);
                                let _ = res.bytes().await.unwrap();
                            }
                        }
                });
            });
        }
    }
    println!("Errors {}/{}", failed.load(Relaxed), counter.load(Relaxed));
}

criterion_group!{
    name = file_handler;
    config = Criterion::default().measurement_time(Duration::from_millis(10_000)).warm_up_time(Duration::from_millis(10));
    targets = filehandler_benchmark
}

criterion_main!(file_handler);
