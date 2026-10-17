// A variable bound over a tuple pattern has the pattern's type, as dotty's `typedBind` takes it:
// over an `Any` a tuple of the elements, whose `_1` is selected; over a tuple the tuple itself. A
// member pattern definition over an `Any` reads its holder's elements so (`makePatDef`'s
// `MatchableTuple`, `val (foo, bar) = null: Any` of dotty's pos/trailingCommas; `.runtimeChecked` here only
// silences the refutability warning scalac gives too).
package patternbindtupletype

object Defs:
  val (foo, bar) = ((1, "x"): Any).runtimeChecked

object Main:
  def main(args: Array[String]): Unit =
    val v: Any = (1, "x")
    val t = v match { case x @ (a, b) => x }
    println(t._1)
    val p: (Int, String) = (2, "y")
    val u = p match { case x @ (a, b) => x }
    println(u._2.length)
    println(s"${Defs.foo} ${Defs.bar}")
