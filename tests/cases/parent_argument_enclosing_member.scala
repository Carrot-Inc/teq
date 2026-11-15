// The arguments of a trait a class extends stand in that class's constructor and are typed in the scope around the
// class: a member of the object, the class or the trait around is read from that scope's instance, whichever
// class, object or anonymous class extends the trait.
trait Labelled(val label: Int)
trait Boxed[A](val item: A)

object Depot:
  private var count: Int = 0
  val base: Int = 40
  lazy val late: Int = 50
  def raise(k: Int): Int = k + 1
  private def hidden(k: Int): Int = k + 2
  class ByMethod extends Labelled(raise(1))
  class ByValue extends Labelled(base)
  class ByLazy extends Labelled(late)
  class ByPrivate extends Labelled(hidden(1))
  class Counted extends Labelled({ count += 1; count })
  object Single extends Labelled(raise(5))
  class Typed extends Boxed[Int](raise(6))
  class Inferred extends Boxed(raise(7))
  class WithParam(v: Int) extends Labelled(raise(v))
  case class Recorded(v: Int) extends Labelled(raise(v))
  def anonymous(): Int = (new Labelled(raise(8)) {}).label

class Tools:
  def inherited(k: Int): Int = k + 100

class Yard(step: Int) extends Tools:
  self =>
  def raise(k: Int): Int = k + step
  val base: Int = step * 10
  object Kit { def twice(k: Int): Int = k * 2 + step }
  trait Tagged(val tag: Int)
  class Inner extends Tagged(raise(1))
  class ByThis extends Tagged(Yard.this.raise(2))
  class ByAlias extends Tagged(self.raise(3))
  class ByParameter extends Tagged(step)
  class ByInherited extends Tagged(inherited(1))
  class ByKit extends Tagged(Kit.twice(4))
  object Lone extends Tagged(base)
  abstract class Open extends Tagged(raise(6))
  class Leaf extends Open
  def local(): Int =
    class Block extends Tagged(raise(3))
    new Block().tag
  def anonymous(): Int = (new Tagged(raise(4)) {}).tag
  class Deep:
    def twice(k: Int): Int = k * 2
    class Far extends Tagged(twice(raise(5)))
    object FarLone extends Tagged(twice(base))

trait Shelf:
  def width: Int
  trait Marked(val mark: Int)
  class Slot extends Marked(width + 1)

object Test:
  def main(args: Array[String]): Unit =
    println(s"${new Depot.ByMethod().label} ${new Depot.ByValue().label} ${new Depot.ByLazy().label} ${new Depot.ByPrivate().label}")
    println(s"${new Depot.Counted().label} ${new Depot.Counted().label} ${Depot.Single.label} ${new Depot.Typed().item} ${new Depot.Inferred().item}")
    println(s"${new Depot.WithParam(9).label} ${Depot.Recorded(3).label} ${Depot.Recorded(3).copy(v = 5).label} ${Depot.anonymous()}")
    val y = new Yard(2)
    println(s"${new y.Inner().tag} ${new y.ByThis().tag} ${new y.ByAlias().tag} ${new y.ByParameter().tag} ${new y.ByInherited().tag} ${new y.ByKit().tag}")
    println(s"${y.Lone.tag} ${new y.Leaf().tag} ${y.local()} ${y.anonymous()}")
    val d = new y.Deep
    println(s"${new d.Far().tag} ${d.FarLone.tag}")
    val s = new Shelf { def width = 6 }
    println(new s.Slot().mark)
