// A trait declaring its own self type conforms to a parent's self type through it: the check
// waits for the self type to be resolved, as dotty's checks run on the completed class.
trait PackSupport[P]:
  val pack: P
trait Pipeline[P]:
  self: PackSupport[P] =>
  def show = s"pack $pack"
trait Framework[P] extends Pipeline[P]:
  self: PackSupport[P] =>
  def twice = show + " " + show
object Obj extends Framework[Int], PackSupport[Int]:
  val pack = 3

@main def run(): Unit =
  println(Obj.show)
  println(Obj.twice)
