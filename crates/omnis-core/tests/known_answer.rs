//! Known-answer tests against reference implementations, computed independently of this crate.
//!
//! FNV-1a and splitmix64 vectors are the published ones; the PCG32 vector is the first six
//! outputs of the reference `pcg32_srandom_r(42, 54)`; the stream goldens were produced by a
//! separate Python model of ARCHITECTURE.md A14 and pin the derivation formula. Changing any
//! golden here means changing every save and replay, so it is re-baselined deliberately.

use omnis_core::{Dice, Pcg32, StreamName, fnv1a64, splitmix64};

#[test]
fn fnv1a64_published_vectors() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    assert_eq!(fnv1a64(b"party"), 0x0c51_a3a7_a1f9_62f7);
}

#[test]
fn splitmix64_published_vectors() {
    assert_eq!(splitmix64(0), 0xe220_a839_7b1d_cdaf);
    assert_eq!(splitmix64(1), 0x910a_2dec_8902_5cc1);
}

#[test]
fn pcg32_reference_vector() {
    let mut rng = Pcg32::seeded(42, 54);
    let got: [u32; 6] = core::array::from_fn(|_| rng.next_u32());
    assert_eq!(
        got,
        [
            0xa15c_02b7,
            0x7b47_f409,
            0xba1d_3330,
            0x83d2_f293,
            0xbfa4_784b,
            0xcbed_606e
        ]
    );
    assert_eq!(rng.draws(), 6);
}

const SEED: u64 = 0x0123_4567_89ab_cdef;

#[test]
fn stream_derivation_goldens() {
    let cases: [(&str, u64, u64, [u32; 3]); 3] = [
        (
            "party",
            0xf704_6f19_f814_9ceb,
            0xf78a_d6a1_ba03_7305,
            [0x8229_6c73, 0xfa8e_7b7c, 0x8149_b295],
        ),
        (
            "combat",
            0x95db_fb4e_6932_52be,
            0x1300_227c_5327_dc95,
            [0xf1c4_aede, 0xb090_1771, 0xe24f_b40b],
        ),
        (
            "gen:0:0:terrain",
            0xb6a0_a861_71f2_33a5,
            0x8b20_6ee4_58c8_c067,
            [0x42e4_af50, 0x17b9_7e83, 0x5abb_7913],
        ),
    ];
    for (name, state, inc, first3) in cases {
        let mut rng = Pcg32::for_stream(SEED, &StreamName::from(name));
        assert_eq!(rng.state(), state, "{name} state");
        assert_eq!(rng.increment(), inc, "{name} increment");
        let got: [u32; 3] = core::array::from_fn(|_| rng.next_u32());
        assert_eq!(got, first3, "{name} outputs");
    }
}

#[test]
fn bounded_draw_golden() {
    let stream = StreamName::from("party");
    let mut rng = Pcg32::for_stream(SEED, &stream);
    let faces: Vec<u32> = (0..5).map(|_| rng.below(6) + 1).collect();
    assert_eq!(faces, [2, 1, 2, 6, 2]);
    assert_eq!(rng.draws(), 5, "no rejections for these draws");

    let mut rng = Pcg32::for_stream(SEED, &stream);
    let trace = Dice::new(5, 6).roll(&mut rng, &stream).unwrap();
    let dice_faces: Vec<u32> = trace.rolls.iter().map(|r| r.value).collect();
    assert_eq!(dice_faces, faces, "dice and bounded draws agree");
    assert_eq!(trace.total, 13);
}

#[test]
fn time_stream_is_symmetric() {
    use omnis_core::{HolderId, PartyId, RegionId};
    let a = HolderId::Party(PartyId(0));
    let b = HolderId::Region(RegionId(3));
    assert_eq!(StreamName::time(a, b), StreamName::time(b, a));
    assert_eq!(StreamName::time(a, b).0, "time:party:0:region:3");
}
