use std::{collections::BTreeMap, fmt::Debug, str::from_utf8, sync::Mutex};

use bf::{
    engine::{Engine, ProgrammableEngine},
    raw,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IOExample {
    input: &'static [u8],
    output: &'static [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IO {
    Input,
    Output,
}

impl IO {
    fn fingerprint(program: &'static str, input: &'static [u8]) -> &'static [IO] {
        static CACHE: Mutex<BTreeMap<(&'static str, &'static [u8]), &'static [IO]>> =
            Mutex::new(BTreeMap::new());
        let mut cache = CACHE.lock().expect("The lock should never be poisoned");
        *cache.entry((program, input)).or_insert_with(|| {
            let mut engine = bf::engine::raw::Engine::new_from_str(program).unwrap();
            let mut input = input;
            let mut fingerprint = vec![];
            'l: loop {
                match engine.run().unwrap() {
                    bf::engine::StopState::Halted => break 'l,
                    bf::engine::StopState::NeedInput => {
                        let (ch, remainder) = input.split_first().unwrap();
                        input = remainder;
                        engine.give_input(*ch);
                        fingerprint.push(IO::Input)
                    }
                    bf::engine::StopState::HasOutput(_) => fingerprint.push(IO::Output),
                }
            }
            // truncate the inputs after the last output
            let after_last_output = fingerprint
                .iter()
                .enumerate()
                .filter_map(|(i, io)| match io {
                    IO::Input => None,
                    IO::Output => Some(i + 1),
                })
                .last()
                .unwrap_or(0);
            fingerprint.truncate(after_last_output);
            Box::leak(fingerprint.into_boxed_slice())
        })
    }
}

/// General engine testing
fn test_engine<E>(
    program: &'static str,
    IOExample {
        input: full_input,
        output: expected,
    }: IOExample,
) where
    E: Engine + ProgrammableEngine,
    E::Program: TryFrom<raw::Program>,
    <E::Program as TryFrom<raw::Program>>::Error: Debug,
{
    let mut engine =
        E::new_from_str(program).expect("The engine should accept the example programs");
    let mut output = vec![];
    let mut fingerprints = vec![];
    let mut input = full_input;
    'l: loop {
        match engine
            .run()
            .expect("The engine should not error on the example programs")
        {
            bf::engine::StopState::Halted => break 'l,
            bf::engine::StopState::NeedInput => {
                let (ch, remainder) = input
                    .split_first()
                    .expect("The engine should be satisfied with the input");
                input = remainder;
                engine
                    .try_give_input(*ch)
                    .expect("After NeedInput the engine should have no input");
                fingerprints.push(IO::Input);
            }
            bf::engine::StopState::HasOutput(ch) => {
                output.push(ch);
                fingerprints.push(IO::Output);
            }
        }
    }
    // converting into strings to make nice errors
    match [&output, expected].map(from_utf8) {
        [Ok(out), Ok(expected)] => assert_eq!(out, expected),
        [Err(_), Ok(expected)] => panic!("Expected string {expected:?}, got bytes {output:?}"),
        [_, Err(_)] => assert_eq!(output, expected),
    }
    // checking fingerprint
    let expected_fp = IO::fingerprint(program, full_input);
    let fp = &fingerprints[..expected_fp.len()];
    assert_eq!(
        expected_fp, fp,
        "The output matched, but it was out of order with the inputs!"
    )
}

include!(env!("TEST_EXAMPLES"));
