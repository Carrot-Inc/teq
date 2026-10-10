// A wildcard argument is bounded by its parameter's bounds instantiated with the compared type's
// arguments (`TypeComparer.isSubArgs`' `paramBounds`), a generic bound and one naming another
// parameter alike.
trait Flags[A]
class On extends Flags[String]
class Cfg[F <: Flags[String]](val n: Int)
object G { given Cfg[?] = new Cfg[On](8) }
def use(using c: Cfg[? <: Flags[String]]): Int = c.n

class Pair[A, B <: List[A]](val b: B)
def head(p: Pair[Int, ? <: List[Int]]): Int = p.b.head

@main def run(): Unit =
  import G.given
  val c: Cfg[?] = new Cfg[On](7)
  val d: Cfg[? <: Flags[String]] = c
  println(d.n)
  println(use)
  val p: Pair[Int, ?] = new Pair[Int, List[Int]](List(3, 4))
  println(head(p))
