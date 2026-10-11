// Two receivers of one given that are one value are one candidate (dotty's `TermRefSet`, its
// prefixes compared by `=:=`), before any candidate is instantiated: `alias.U` of an
// `alias: O.type` is `O.U`, and so is the companion of `alias.C`.
trait TC[X, Y]
trait Outer:
  object U:
    given mk[X, Y]: TC[X, Y] = new TC[X, Y] {}
  class C
  object C:
    given cmk[X, Y]: TC[X, Y] = new TC[X, Y] {}
object O extends Outer
val alias: O.type = O
@main def run(): Unit =
  println(summon[TC[O.U.type, alias.U.type]] != null)
  println(summon[TC[O.C, alias.C]] != null)
