//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub const GAMMA_LUT_14BIT: [u16; 256] = [
    0, 0, 0, 1, 2, 3, 4, 6, 8, 10, 13, 16, 20, 23, 28, 32, 37, 42, 48, 54, 61, 67, 75, 82, 90, 99,
    108, 117, 127, 137, 148, 159, 170, 182, 195, 207, 221, 234, 249, 263, 278, 294, 310, 326, 343,
    361, 379, 397, 416, 435, 455, 475, 496, 517, 539, 561, 583, 607, 630, 654, 679, 704, 730, 756,
    783, 810, 838, 866, 894, 924, 953, 983, 1014, 1045, 1077, 1110, 1142, 1176, 1210, 1244, 1279,
    1314, 1350, 1387, 1424, 1461, 1499, 1538, 1577, 1617, 1657, 1698, 1739, 1781, 1823, 1866, 1910,
    1954, 1998, 2044, 2089, 2136, 2182, 2230, 2278, 2326, 2375, 2425, 2475, 2525, 2577, 2629, 2681,
    2734, 2787, 2841, 2896, 2951, 3007, 3063, 3120, 3178, 3236, 3295, 3354, 3414, 3474, 3535, 3596,
    3658, 3721, 3784, 3848, 3913, 3978, 4043, 4110, 4176, 4244, 4312, 4380, 4449, 4519, 4589, 4660,
    4732, 4804, 4876, 4950, 5024, 5098, 5173, 5249, 5325, 5402, 5479, 5557, 5636, 5715, 5795, 5876,
    5957, 6039, 6121, 6204, 6287, 6372, 6456, 6542, 6628, 6714, 6801, 6889, 6978, 7067, 7156, 7247,
    7337, 7429, 7521, 7614, 7707, 7801, 7896, 7991, 8087, 8183, 8281, 8378, 8477, 8576, 8675, 8775,
    8876, 8978, 9080, 9183, 9286, 9390, 9495, 9600, 9706, 9812, 9920, 10027, 10136, 10245, 10355,
    10465, 10576, 10688, 10800, 10913, 11027, 11141, 11256, 11371, 11487, 11604, 11721, 11840,
    11958, 12078, 12198, 12318, 12440, 12562, 12684, 12807, 12931, 13056, 13181, 13307, 13433,
    13561, 13688, 13817, 13946, 14076, 14206, 14337, 14469, 14602, 14735, 14868, 15003, 15138,
    15273, 15410, 15547, 15685, 15823, 15962, 16102, 16242, 16383,
];

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");
}
