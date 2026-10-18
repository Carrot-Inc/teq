// teq: --source 3.3
// `Types.derivesFrom` takes a union only where both sides derive, a `Null` or `Nothing` side aside, and a type
// parameter by its bound: `P | Int` and a parameter bounded by `Int => Int` (no `PolyFunction`, and no `FunctionN`
// itself) are refused, `P | Null` is not.
type P = [A] => A => A
def t1(f: P | Int) = f _
def t2[F <: Int => Int](f: F) = f _
def t3(f: P | Null) = f _
// expect: 6:22: error: Only function types can be followed by _ but the current expression has type
// expect: 7:33: error: Only function types can be followed by _ but the current expression has type F
// absent: 8:
