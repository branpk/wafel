use wafel_api::Timeline;

fn main() {
    let mut timeline = unsafe { Timeline::new("libsm64/sm64_us.dll") };

    assert_eq!(
        timeline.read(1500, "gCurrLevelNum"),
        timeline.constant("LEVEL_BOWSER_1")
    );

    for frame in 0..1000 {
        if frame % 2 == 1 {
            timeline.write(
                frame,
                "gControllerPads[0].button",
                timeline.constant("START_BUTTON"),
            );
        }
    }

    assert_eq!(
        timeline.read(1500, "gCurrLevelNum"),
        timeline.constant("LEVEL_CASTLE_GROUNDS")
    );

    for frame in 1500..2000 {
        if frame % 2 == 1 {
            timeline.write(
                frame,
                "gControllerPads[0].button",
                timeline.constant("A_BUTTON"),
            );
        }
    }

    assert_eq!(
        timeline.read(2500, "gMarioState.action"),
        timeline.constant("ACT_IDLE")
    );
    timeline.write(2450, "gMarioState.pos[1]", 5000.0.into());
    assert_eq!(
        timeline.read(2500, "gMarioState.action"),
        timeline.constant("ACT_FREEFALL")
    );

    eprintln!("{:08X}", timeline.read(2500, "gMarioState.action").as_int());
    eprintln!("{:08X}", timeline.constant("ACT_ID_MASK").as_int());
    eprintln!(
        "{:08X}",
        timeline
            .read(2500, "gMarioState.action & ACT_ID_MASK")
            .as_int()
    );
}
