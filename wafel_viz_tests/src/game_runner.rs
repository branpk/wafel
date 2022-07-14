use wafel_api::{load_m64, Game, Input, SaveState};

#[derive(Debug, Default)]
pub struct GameRunner {
    ctx: Option<Context>,
}

impl GameRunner {
    pub fn get_frame(
        &mut self,
        game_version: &'static str,
        m64: &'static str,
        frame: u32,
    ) -> &Game {
        if self.ctx.as_ref().map(|c| (c.game_version, c.m64)) != Some((game_version, m64)) {
            self.ctx = None;
        }
        let ctx = self.ctx.get_or_insert_with(|| {
            let game = unsafe { Game::new(&format!("libsm64/sm64_{}", game_version)) };
            let power_on = game.save_state();
            let (_, inputs) = load_m64(&format!("wafel_viz_tests/input/{}.m64", m64));
            Context {
                game_version,
                m64,
                game,
                power_on,
                inputs,
            }
        });
        let game = &mut ctx.game;
        if game.frame() > frame {
            game.load_state(&ctx.power_on);
        }
        while game.frame() < frame {
            let input = ctx
                .inputs
                .get(game.frame() as usize)
                .cloned()
                .unwrap_or_default();
            game.set_input(input);
            game.advance();
        }
        game
    }
}

#[derive(Debug)]
struct Context {
    game_version: &'static str,
    m64: &'static str,
    game: Game,
    power_on: SaveState,
    inputs: Vec<Input>,
}
