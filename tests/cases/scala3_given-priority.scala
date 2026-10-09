// Adapted from scala3 tests/pos/given-priority.scala (Apache-2.0, see tests/scala3/README.md); the schemes that fit the subset, with the results printed.
//> using scala 3.8.4
trait A { def name: String }
trait B extends A
class AI extends A { def name = "A" }
class BI extends B { def name = "B" }
trait LowPriorityImplicits:
  given g1: A = AI()
object NormalImplicits extends LowPriorityImplicits:
  given g2: B = BI()
def test1 =
  import NormalImplicits.given
  val x = summon[A]
  val y: B = x
  println(x.name)
  println(summon[B].name)
// Second scheme: prioritize with a nested import
object Priority:
  object Low:
    given g1: A = AI()
  object High:
    given g2: B = BI()
def test2 =
  import Priority.Low.given
  def inner =
    import Priority.High.given
    println(summon[A].name)
  inner
  println(summon[A].name)
// Third scheme: most general type wins between givens of the same owner (3.7)
object General:
  given g1: A = AI()
  given g2: B = BI()
def test3 =
  import General.given
  println(summon[A].name)
  println(summon[B].name)
@main def main(): Unit =
  test1
  test2
  test3
