use criterion::{criterion_group, criterion_main, Criterion};
use futures::stream::StreamExt;
use rand::Rng;
use rmesg::{
    entry::{Entry, LogFacility, LogLevel},
    klogctl::{klog, KLogEntries},
    kmsgfile::{kmsg, KMsgEntriesIter, KMsgEntriesStream},
};
use std::hint::black_box;
use std::time::Duration;

fn generate_random_usize() -> usize {
    let mut rng = rand::rng();
    rng.random_range(0..usize::MAX)
}

fn generate_random_bool() -> bool {
    let mut rng = rand::rng();
    rng.random_bool(0.5)
}

fn generate_random_duration() -> Duration {
    let mut rng = rand::rng();
    Duration::from_secs_f64(rng.random::<f64>())
}

fn random_entry() -> Entry {
    Entry {
        timestamp_from_system_start: match generate_random_bool() {
            true => Some(generate_random_duration()),
            false => None,
        },
        facility: match generate_random_bool() {
            true => Some(LogFacility::Kern),
            false => None,
        },
        level: match generate_random_bool() {
            true => Some(LogLevel::Info),
            false => None,
        },
        sequence_num: match generate_random_bool() {
            true => Some(generate_random_usize()),
            false => None,
        },
        message: "Some very long string with no purpose. Lorem. Ipsum. Something Something."
            .to_owned(),
    }
}

fn display_entry() {
    let displayed = format!("{}", random_entry());
    black_box(displayed);
}

fn entry_to_kmsg_str() {
    let displayed = random_entry().to_kmsg_str().unwrap();
    black_box(displayed);
}

fn entry_to_klog_str() {
    let displayed = random_entry().to_klog_str().unwrap();
    black_box(displayed);
}

fn kmsg_read() {
    let file = match generate_random_bool() {
        true => Some("/dev/kmsg".to_owned()),
        false => None,
    };
    let entries = kmsg(file).unwrap();
    black_box(entries);
}

fn kmsg_iter_read() {
    let file = match generate_random_bool() {
        true => Some("/dev/kmsg".to_owned()),
        false => None,
    };
    let entries = KMsgEntriesIter::with_options(file, generate_random_bool()).unwrap();
    let mut count = 0;
    for entry in entries {
        black_box(entry).unwrap();
        count += 1;
        if count > 25 {
            break;
        }
    }
}

async fn kmsg_stream_read() {
    let file = match generate_random_bool() {
        true => Some("/dev/kmsg".to_owned()),
        false => None,
    };
    let mut entries = KMsgEntriesStream::with_options(file, generate_random_bool())
        .await
        .unwrap();
    let mut count = 0;
    while let Some(entry) = entries.next().await {
        black_box(entry).unwrap();
        count += 1;
        if count > 25 {
            break;
        }
    }
}

fn klog_read() {
    let entries = klog(false).unwrap();
    black_box(entries);
}

fn klog_iter_read() {
    let entries = KLogEntries::with_options(false, Duration::from_secs(1)).unwrap();
    let mut count = 0;
    for entry in entries {
        black_box(entry).unwrap();
        count += 1;
        if count > 25 {
            break;
        }
    }
}

async fn klog_stream_read() {
    let mut entries = KLogEntries::with_options(false, Duration::from_secs(1)).unwrap();
    let mut count = 0;
    while let Some(entry) = StreamExt::next(&mut entries).await {
        black_box(entry).unwrap();
        count += 1;
        if count > 25 {
            break;
        }
    }
}

pub fn benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("display_entry", |b| {
        b.iter(|| {
            display_entry();
            black_box(());
        })
    });
    c.bench_function("entry_to_kmsg_str", |b| {
        b.iter(|| {
            entry_to_kmsg_str();
            black_box(());
        })
    });
    c.bench_function("entry_to_klog_str", |b| {
        b.iter(|| {
            entry_to_klog_str();
            black_box(());
        })
    });

    c.bench_function("kmsg_read", |b| {
        b.iter(|| {
            kmsg_read();
            black_box(());
        })
    });
    c.bench_function("kmsg_iter_read", |b| {
        b.iter(|| {
            kmsg_iter_read();
            black_box(());
        })
    });
    c.bench_function("kmsg_stream_read", |b| {
        b.to_async(&rt).iter(|| async {
            kmsg_stream_read().await;
            black_box(());
        });
    });

    c.bench_function("klog_read", |b| {
        b.iter(|| {
            klog_read();
            black_box(());
        })
    });
    c.bench_function("klog_iter_read", |b| {
        b.iter(|| {
            klog_iter_read();
            black_box(());
        })
    });
    c.bench_function("klog_stream_read", |b| {
        b.to_async(&rt).iter(|| async {
            klog_stream_read().await;
            black_box(());
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
