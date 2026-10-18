// An infix operation's parenthesized assignment to a selection (`migrateInfixOp`'s `Parens(Assign(..))`), under
// 3.6-migration the application of the assignment (`needsPatch`): the assignment made once, its unit passed. The
// warning is silenced.
// teq: --source 3.6-migration --wconf msg=infix.named.arguments.lists:s
class C {
  var x: Int = 0
  var calls = 0
  infix def f(u: Unit): Int = { calls += 1; x }
}
object P {
  def main(args: Array[String]): Unit = {
    val c = new C
    println(c f (c.x = 4))
    println(c.calls)
  }
}
