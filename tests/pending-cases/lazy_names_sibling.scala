//> using platform jvm
// Pending on lazy-val-names (queued): an unused local class inside a local method is gone before dotty's LazyVals
// meets the unit, where teq's walk counts its lazy member; teq's names are unique and the same on every run,
// scalac's numbers are the expectation, here 2 for the class's own `x`.
// A local method declaring an unused local class with a lazy member, a local lazy val beside it, then the enclosing
// class's lazy member: the declared fields of the class name its holder by the unit's counter.
class Outer:
  def go(): Unit =
    def f(): Int =
      class Unused { lazy val x = 1 }
      1
    lazy val x = 2
    println(x)
    println(f())
  lazy val x = 3
object Main:
  def main(args: Array[String]): Unit =
    val o = new Outer
    o.go()
    println(o.x)
    println(classOf[Outer].getDeclaredFields.map(_.getName).sorted.mkString(","))
