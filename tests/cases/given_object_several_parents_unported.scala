// teq: --source 3.6
// Under 3.6, whose comparison of givens teq does not port (3.6 and 3.7-migration compare as teq did before it told
// the versions apart), a given object of several parents is read as its first parent, `P`, and preferred over a given
// of `P & Q`: teq prints `1`. scalac 3.8.4 prints `1` too, with a warning (E205) that 3.7 changes the choice, which
// teq does not give at 3.6.
class P { def x: Int = 1 }
trait Q
trait R extends Q
object O:
  given pair: P() with R with {}
  given other: (P & Q) = new P with Q { override def x = 2 }
@main def run(): Unit =
  import O.given
  println(summon[P].x)
