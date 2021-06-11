use wafel_api::Game;

fn main() {
    let mut game = unsafe { Game::new("libsm64/sm64_us.dll") };

    let power_on = game.save_state();

    game.advance_n(1500);
    assert_eq!(game.read("gCurrLevelNum"), game.constant("LEVEL_BOWSER_1"));

    game.load_state(&power_on);
    for frame in 0..1000 {
        if frame % 2 == 1 {
            game.write("gControllerPads[0].button", game.constant("START_BUTTON"));
        }
        game.advance();
    }

    game.advance_n(500);
    assert_eq!(
        game.read("gCurrLevelNum"),
        game.constant("LEVEL_CASTLE_GROUNDS")
    );

    for frame in 1500..2000 {
        if frame % 2 == 1 {
            game.write("gControllerPads[0].button", game.constant("A_BUTTON"));
        }
        game.advance();
    }

    game.advance_n(450);
    let state = game.save_state();

    game.advance_n(50);
    assert_eq!(game.read("gMarioState.action"), game.constant("ACT_IDLE"));

    game.load_state(&state);
    game.write("gMarioState.pos[1]", 5000.0.into());
    game.advance_n(50);
    assert_eq!(
        game.read("gMarioState.action"),
        game.constant("ACT_FREEFALL")
    );
}
