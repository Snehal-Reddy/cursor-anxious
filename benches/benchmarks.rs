use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use evdev::{EventType, InputEvent, RelativeAxisCode};
use mouse_scroll_daemon::{AnxiousParams, AnxiousState, apply_anxious_scroll, process_events};
use std::hint::black_box;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Helper function to create AnxiousState with a specific timestamp
fn create_anxious_state_with_time(prev_time: SystemTime) -> AnxiousState {
    AnxiousState { prev_time }
}

fn create_test_events() -> Vec<InputEvent> {
    vec![
        // High-res wheel events (these get processed) - with proper timestamps
        InputEvent::new_now(
            EventType::RELATIVE.0,
            RelativeAxisCode::REL_WHEEL_HI_RES.0,
            120,
        ),
        InputEvent::new_now(
            EventType::RELATIVE.0,
            RelativeAxisCode::REL_WHEEL_HI_RES.0,
            240,
        ),
        InputEvent::new_now(
            EventType::RELATIVE.0,
            RelativeAxisCode::REL_WHEEL_HI_RES.0,
            360,
        ),
        // Regular wheel events (these get dropped)
        InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_WHEEL.0, 1),
        InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_WHEEL.0, -1),
        // Other events (these get passed through)
        InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, 10),
        InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, 5),
    ]
}

fn benchmark_apply_anxious_scroll(c: &mut Criterion) {
    let mut group = c.benchmark_group("apply_anxious_scroll");

    // Test different velocity scenarios
    let scenarios = vec![
        ("slow_scroll", 1.0, Duration::from_millis(100)), // 10 units/sec
        ("medium_scroll", 5.0, Duration::from_millis(50)), // 100 units/sec
        ("fast_scroll", 15.0, Duration::from_millis(10)), // 1500 units/sec
        ("very_fast_scroll", 50.0, Duration::from_millis(5)), // 10000 units/sec
    ];

    for (name, value, elapsed) in scenarios {
        group.bench_with_input(
            BenchmarkId::new("velocity_scenarios", name),
            &(value, elapsed),
            |b, (value, elapsed)| {
                let params = AnxiousParams::default();
                // Use a fixed timestamp to avoid SystemTime overflow issues
                let base_time = UNIX_EPOCH + Duration::from_secs(1000000000); // Far in the future
                let mut state = create_anxious_state_with_time(base_time);
                let timestamp = base_time + *elapsed; // Pre-compute timestamp outside hot path

                b.iter(|| {
                    // Only measure the hot path: apply_anxious_scroll call
                    black_box(apply_anxious_scroll(
                        black_box(*value),
                        black_box(timestamp),
                        black_box(&params),
                        black_box(&mut state),
                    ))
                })
            },
        );
    }

    // Test different parameter configurations
    let param_configs = vec![
        ("default", AnxiousParams::default()),
        (
            "high_sensitivity",
            AnxiousParams {
                base_sens: 1.0,
                max_sens: 30.0,
                ramp_up_rate: 0.5,
            },
        ),
        (
            "low_sensitivity",
            AnxiousParams {
                base_sens: 0.5,
                max_sens: 5.0,
                ramp_up_rate: 0.1,
            },
        ),
    ];

    for (name, params) in param_configs {
        group.bench_with_input(
            BenchmarkId::new("parameter_configs", name),
            &params,
            |b, params| {
                let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
                let mut state = create_anxious_state_with_time(base_time);
                let timestamp = base_time + Duration::from_millis(10); // Pre-compute timestamp outside hot path

                b.iter(|| {
                    // Only measure the hot path: apply_anxious_scroll call
                    black_box(apply_anxious_scroll(
                        black_box(10.0),
                        black_box(timestamp),
                        black_box(params),
                        black_box(&mut state),
                    ))
                })
            },
        );
    }

    group.finish();
}

fn benchmark_event_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_processing");

    // Test different batch sizes - simplified to avoid timestamp issues
    let batch_sizes = vec![1, 5, 10, 20, 50];

    for size in batch_sizes {
        group.bench_with_input(BenchmarkId::new("batch_size", size), &size, |b, &size| {
            let events = create_test_events()
                .into_iter()
                .cycle()
                .take(size)
                .collect::<Vec<_>>();
            let params = AnxiousParams::default();

            b.iter(|| {
                // Create a simple state that won't cause timestamp issues
                let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
                let mut state_clone = create_anxious_state_with_time(base_time);

                // Use the actual process_events function - this is the real hot path
                black_box(process_events(
                    black_box(events.iter().cloned()),
                    black_box(&params),
                    black_box(&mut state_clone),
                ))
            })
        });
    }

    // Test realistic event processing with proper timestamps
    group.bench_function("realistic_event_processing", |b| {
        let events = create_test_events();
        let params = AnxiousParams::default();
        let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);

        b.iter(|| {
            let mut state_clone = create_anxious_state_with_time(base_time);
            // Use the actual process_events function with proper timestamps
            black_box(process_events(
                black_box(events.iter().cloned()),
                black_box(&params),
                black_box(&mut state_clone),
            ))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_apply_anxious_scroll,
    benchmark_event_processing
);
criterion_main!(benches);
