use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{rngs::SmallRng, Rng as _, SeedableRng as _};
use rate_limiter::RateLimiter;
use snmalloc_rs::SnMalloc;
use tokio::{runtime, task::yield_now};

#[global_allocator]
static ALLOC: SnMalloc = SnMalloc;

const SLOTS: usize = 10;
const LIMIT: usize = 1_000;

fn rate_limiter(c: &mut Criterion) {
    let rt = runtime::Builder::new_multi_thread().build().unwrap();
    rt.block_on(async move {
        benchmark_group(c, &mut "mostly-passing", |rate_limiter, limited| {
            if limited {
                rate_limiter.rotate_slots();
            }
        })
        .await;
        benchmark_group(c, &mut "mostly-failing", |_, _| {}).await;
    })
}

async fn benchmark_group(c: &mut Criterion, name: &str, f: impl Fn(&RateLimiter<u32>, bool)) {
    let mut rng = SmallRng::from_entropy();
    let mut group = c.benchmark_group(name);
    for key_space in [10, 100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("key-space", key_space),
            &key_space,
            |b, &key_space| {
                let rate_limiter = RateLimiter::<u32>::new(LIMIT, SLOTS);
                b.iter(|| {
                    let key = rng.gen_range(0..key_space);
                    let limited = rate_limiter.check_limited(key);
                    black_box(limited);
                    f(&rate_limiter, limited);
                });
            },
        );
    }
    group.finish();

    let mut group = c.benchmark_group(&format!("{}-contention", name));
    for tasks in (0..9).map(|i| i * 2) {
        group.bench_with_input(BenchmarkId::new("tasks", tasks), &tasks, |b, &tasks| {
            let rate_limiter = RateLimiter::<u32>::new(LIMIT, SLOTS);
            let tasks: Vec<_> = (0..tasks)
                .map(|_| {
                    let rate_limiter = rate_limiter.clone();
                    tokio::spawn(async move {
                        loop {
                            rate_limiter.check_limited(0);
                            yield_now().await;
                        }
                    })
                })
                .collect();
            b.iter(|| {
                let limited = rate_limiter.check_limited(0);
                black_box(limited);
                f(&rate_limiter, limited);
            });
            for task in tasks {
                task.abort();
            }
        });
    }
    group.finish();
}

criterion_group!(benches, rate_limiter);
criterion_main!(benches);
