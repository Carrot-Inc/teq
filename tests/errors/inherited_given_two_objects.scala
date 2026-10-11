// expect: 16:48: error: ambiguous given instances for Show[Y]: Y.s, X.s
// expect: 1 error found
// A trait's given that two objects of the implicit scope inherit is two references, `X.s` and
// `Y.s` (dotty's `TermRefSet`, a symbol under each prefix), which rank alike: an ambiguity.
trait Show[-A]:
  def show: String
trait Instances:
  val tag: String
  given s: Show[X] = new Show[X] { def show = "s " + tag }
class X
object X extends Instances:
  val tag = "X"
class Y extends X
object Y extends Instances:
  val tag = "Y"
@main def run(): Unit = println(summon[Show[Y]].show)
