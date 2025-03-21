#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
enum GameMode {
  #[default]
  PvP,
  Defuse,
}