use std::fmt::Debug;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use bf::{
    engine::{Engine, ProgrammableEngine},
    raw,
};

/// General engine benching
fn bench_engine<E>(
    c: &mut Criterion,
    source: &str,
    io_example: &str,
    engine_name: &str,
    program: &'static str,
    input: &[u8],
) where
    E: Engine + ProgrammableEngine + Clone,
    E::Program: TryFrom<raw::Program>,
    <E::Program as TryFrom<raw::Program>>::Error: Debug,
{
    let engine = E::new_from_str(program).expect("The engine should accept the example programs");
    c.bench_with_input(
        BenchmarkId::new(format!("{source}/{engine_name}"), io_example),
        &input,
        |b, input| {
            b.iter(|| {
                let mut input = *input;
                let mut engine = engine.clone();
                'l: loop {
                    match engine.run().unwrap() {
                        bf::engine::StopState::Halted => break 'l,
                        bf::engine::StopState::NeedInput => {
                            let (ch, remainder) = input.split_first().unwrap();
                            input = remainder;
                            engine.give_input(*ch);
                        }
                        bf::engine::StopState::HasOutput(ch) => {
                            black_box(ch);
                        }
                    }
                }
            })
        },
    );
}

include!(env!("BENCH_EXAMPLES"));
