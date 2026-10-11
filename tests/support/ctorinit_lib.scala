// The jar side of tests/cases/macro_jar_ctor_val, compiled by scalac: a class and a trait whose
// constructors' statements call a method reading a val the template defines before them and
// before another field (izumi-reflect's boopickle `IdentMap3Plus`, whose `updated` reads
// `maxDepth`), which teq types when a run first reads it, in the middle of the construction.
package ctorinitlib

final class Recorder(first: String, second: String, third: String):
  val sep: String = "/"
  var log: String = ""
  add(first)
  add(second)
  add(third)
  def add(s: String): Unit = log = log + sep + s

trait Counting:
  val step: Int = 10
  var seen: List[Int] = Nil
  note(1)
  note(2)
  note(3)
  def note(n: Int): Unit = seen = (n * step) :: seen

final class Counter extends Counting
