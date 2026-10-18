// teq: --source 3.3
// A union's side that is no bottom type (`isBottomType` of a type bounded by `AnyRef`) leaves `derivesFrom` asking both
// sides, so `P | N` is refused; a cyclic bound, which a walk meets again on its own path, derives nothing.
type P = [A] => A => A
def t1[N <: AnyRef](f: P | N) = f _
def t2[F <: G, G <: F](f: F) = f _
def t3[N <: Null](f: P | N) = f _
// expect: 5:33: error: Only function types can be followed by _ but the current expression has type
// expect: 6:32: error: Only function types can be followed by _ but the current expression has type F
// absent: 7:
