// A wildcard given import from a path whose type is two instances of the member's owner (`x: IB & SB`): `import
// x.given` brings `gv: List[Int & String]` (the member's type seen from `x`, `List[A]` at +1), which scalac 3.8.4's
// `summon[List[Int & String]]` finds. teq finds no given (master the same); with a named import (`import x.gv`), a
// trait `Both extends IB with SB` or one instance it finds it. The closure adversary of the pass on 97f0487ac (round 24).
trait Box[A]:
  given gv: List[A] = Nil
trait IB extends Box[Int]
trait SB extends Box[String]
def t(x: IB & SB): Int =
  import x.given
  summon[List[Int & String]].length

@main def run(): Unit = println("ok")
