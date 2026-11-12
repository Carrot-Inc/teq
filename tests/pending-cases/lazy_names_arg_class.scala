//> using platform jvm
// Pending on lazy-val-names (queued): teq's walk of a file's lazy definitions meets an anonymous class's superclass
// arguments twice (with the `New`'s arguments and again with the class's parents), where dotty's MegaPhase meets each
// surviving definition once; teq's names are unique and the same on every run, scalac's numbers are the
// expectation, here 3 for the class's own `x`.
// A lazy val in an anonymous class's superclass argument and a lazy member of the anonymous class, then the
// enclosing class's lazy member: the declared fields of the enclosing class name its holder by the unit's counter.
class Base(val n: Int)
class Outer:
  def go(): Unit =
    val b = new Base({ lazy val x = 1; x }) { lazy val x = 2 }
    println(b.n)
  lazy val x = 3
object Main:
  def main(args: Array[String]): Unit =
    val o = new Outer
    o.go()
    println(o.x)
    println(classOf[Outer].getDeclaredFields.map(_.getName).sorted.mkString(","))
