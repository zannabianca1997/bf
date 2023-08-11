use std::{fmt::Debug, str::from_utf8};

use bf::{
    engine::{Engine, ProgrammableEngine},
    raw,
};

struct IOExample {
    input: &'static [u8],
    output: &'static [u8],
}

/// General engine testing
fn test_engine<E>(
    program: &'static str,
    IOExample {
        mut input,
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
                    .expect("After NeedInput the engine should have no input")
            }
            bf::engine::StopState::HasOutput(ch) => {
                output.push(ch);
            }
        }
    }
    // converting into strings to make nice errors
    match [&output, expected].map(from_utf8) {
        [Ok(out), Ok(expected)] => assert_eq!(out, expected),
        [Err(_), Ok(expected)] => panic!("Expected string {expected:?}, got bytes {output:?}"),
        [_, Err(_)] => assert_eq!(output, expected),
    }
}

include!(env!("TEST_EXAMPLES"));
