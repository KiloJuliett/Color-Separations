use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use std::fs::remove_file;
use std::process::Command;
use std::time::Duration;
use std::time::Instant;

pub fn bench_separations(criterion: &mut Criterion) {
    let arguments = "-p sRGB -o benches/output.cube -c 102 51 153 -s 16 -t 10000";

    let mut group = criterion.benchmark_group("bench_separations");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(1000));
    
    group.bench_function("bench_separations", |bencher| bencher.iter_custom(|iterations| {
        let mut duration = Duration::new(0, 0);

        for _ in 0..iterations {
            let mut process = Command::new("cargo");
            process.args(&["run", "--"]);
            process.args(arguments.split(' '));

            let start = Instant::now();

            let _ = process.output();

            duration += start.elapsed();

            // Clean up
            let _ = remove_file("benches/output.cube");
            for index in 0..=10 {
                let _ = remove_file(format!("benches/output_{}.cube", index));
                let _ = remove_file(format!("benches/output_{}m.cube", index));
            }
        }

        duration
    }));

    group.finish();
}

criterion_group!(benches, bench_separations);
criterion_main!(benches);