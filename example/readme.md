# Steps to run RUG on your crate

In this doc, we use an example crate, [humantime](https://github.com/chronotope/humantime) to show how to run RUG on your crate.


## 1. Clone repo and check version

Clone the repo and since RUG is binded to toolchain nightly-2022-12-10, we checkout human time's `12ce6f50894a56a410b390e5608ac9db8afe2407` version.

Remove several lint flags in `lib.rs` or `mod.rs` to ensure RUG can run on it. Like [lib.rs](./humantime/src/lib.rs):

        /*
        #![forbid(unsafe_code)]
        #![warn(missing_debug_implementations)]
        #![warn(missing_docs)]
        */


## 2. Build and install cargo tools

+ for RUG, please `cargo install --path .` for safefinder
+ bolero, please `cargo install cargo-bolero@0.8.0`

## 3. Run llm

launch [main.py](./main.py) on the target crate, it will do the analysis and request GPT to write test. Don't forget to add your api key to OpenAI client (2 places in the script).

**The finished crate should be like [humantime.bk](./humantime.bk), strongly recommended to backup this to avoid further requests for LLM**

## 4.(Fuzz) Transform unit test to fuzz driver

1. Build [fuzz_transform](./fuzz_transform/)
2. Launch [fuzz_transform.py](./fuzz_transform.py) with binary:

        python3 fuzz_transform.py crate-fd ./fuzz_transform/target/debug/fuzz_transform

3. It's expected to have [xxx_fuzz_trans.json](./humantime/humantime_fuzz_trans.json)

## 5. Apply bolero configuration

Run the [apply_bolero.py](./apply_driver.py), simpled add bolero dependency to your crate


**Notice: the backtrace crate is updated, it's expected to fail to compile, run: cargo update -p backtrace@0.3.75 --precise 0.3.68**

**Please ensure the crate can be built after this step**

## 6. Apply fuzzing driver transformation

Run [apply_driver.py](./apply_driver.py), the expected result should be like [humantime](./humantime/):

1. There is an [inputs](./humantime/inputs/) folder containing the initial fuzzing corpus input
2. The test is converted to fuzzing target like below:

        #[cfg(test)]
        mod tests_rug_11 {
            use super::*;
            use crate::{FormattedDuration, format_duration};
            use std::time::Duration;
            #[test]
            fn test_rug() {

            extern crate bolero;
            extern crate arbitrary;
            bolero::check!()
                .for_each(|rug_data| {
                    if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

                let duration = Duration::from_secs(rug_fuzz_0);
                let p0 = format_duration(duration);
                <FormattedDuration>::get_ref(&p0);
                    }
        });    }
        }
3. Use [bolero](https://github.com/camshaft/bolero) to build the fuzzing target 


## 7. Optional batch run:

Use the [fuzz_engine.py](https://github.com/CXWorks/rug-ae/blob/main/rug_ae1/source/rug-gpt/fuzz_engine.py) to launch all the fuzzing in the transformed crate


**By default RUG has 60 seconds timeout for each target, you can change it to longer time for bug hunting, etc**