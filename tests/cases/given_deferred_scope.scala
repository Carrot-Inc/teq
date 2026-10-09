// Which members the implementation of a deferred given is, as dotty's `implementDeferredGivens`
// takes them: a class's `using` parameters serve the search, not an old-style `implicit` clause; two
// unrelated traits declaring one deferred given share one implementation, whichever's signature was
// completed first (teq reported conflicting members); a lazy val or a narrower deferred given of a
// trait further down re-declares it.
import scala.compiletime.deferred

trait T:
  given x: Int = deferred
object Site:
  given Int = 99
  class OldStyle(implicit n: Int) extends T
  class NewStyle(using n: Int) extends T

trait A:
  given y: Int = deferred
trait B:
  given y: Int = deferred
class Both(using Int) extends A with B

trait L:
  given z: Int = deferred
  def read: Int = z
trait M extends L:
  override lazy val z: Int = 23
class ByLazy extends M

trait Wide:
  given w: Any = deferred
trait Narrow extends Wide:
  override given w: String = deferred
class Narrowed(using String) extends Narrow

@main def run(): Unit =
  println(s"${new Site.OldStyle()(using 12).x} ${new Site.NewStyle(using 12).x}")
  val both = new Both(using 17)
  println(s"${(both: A).y} ${(both: B).y} ${both.y}")
  println((new ByLazy).read)
  val n = new Narrowed(using "ok")
  println(s"${n.w} ${(n: Wide).w}")
