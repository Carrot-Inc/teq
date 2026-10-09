// expect: 25:11: error: cannot reduce summonFrom with
// expect:  patterns :  case x: Int if n > 0
// expect: 26:11: error: cannot reduce summonFrom with
// expect:  patterns :  case x: Int if x > 2
// expect: 2 errors found
// A guard of a `summonFrom` case that is no constant where the body expands stops the
// reduction, as an inline match's does (scalac's `InlineReducer.reduceCase`): `n > 0` over a
// runtime argument, `x > 2` over the given the case found, whose value is no constant; a
// constant argument folds it (`guarded(1)` is 3). scalac 3.8.4 reports the same two errors,
// naming the patterns as it prints them (`case given x @ _:Int if n > 0`).
import scala.compiletime.summonFrom
object Lib:
  inline def guarded(n: Int): Int = summonFrom {
    case x: Int if n > 0 => x
    case _ => 0
  }
  inline def onBinder: String = summonFrom {
    case x: Int if x > 2 => "big"
    case _ => "small"
  }
@main def run(): Unit =
  given Int = 3
  val n = "ab".length
  println(Lib.guarded(1))
  println(Lib.guarded(n))
  println(Lib.onBinder)
