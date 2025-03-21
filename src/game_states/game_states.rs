use bevy::prelude::*;

#[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
enum GameState {
    #[default]
    Menu,
    InGame {
        paused: bool,
    },
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct InGame;

// While we can simply do OnEnter(GameState::InGame{paused: true}),
// we need to be able to reason about "while we're in the game, paused or not".
// To this end, we define the InGame computed state:
impl ComputedStates for InGame {
    // Computed states can be calculated from one or many source states.
    type SourceStates = GameState;

    // Now, we define the rule that determines the value of our computed state.
    fn compute(sources: GameState) -> Option<InGame> {
        match sources {
            // We can use pattern matching to express the
            //"I don't care whether or not the game is paused" logic!
            GameState::InGame { .. } => Some(InGame),
            _ => None,
        }
    }
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
// This macro means that `GamePhase` will only exist when we're in the `InGame` computed state.
// The intermediate computed state is helpful for clarity here, but isn't required:
// you can manually `impl SubStates` for more control, multiple parent states and non-default initial value!
#[source(InGame = InGame)]
enum GamePhase {
    #[default]
    Setup,
    Battle,
    Conclusion,
}

pub struct GameStatesPlugin;

impl Plugin for GameStatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_computed_state::<InGame>()
            .add_sub_state::<GamePhase>();
    }
}
