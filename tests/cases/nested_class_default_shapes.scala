// Constructor defaults of classes nested in a class, each getter its companion's, an object of each
// enclosing instance (`Desugar.classDef`), reading that instance's state: a by-name parameter's default
// evaluated at each read and passed through the creation's prefix like any other
// (`Applications.findDefaultGetter`), a function over the instance, a private member of the enclosing
// class, the class's type parameter, a class nested two deep, a parent's call that passes nothing, and
// a written companion nothing but the default reaches, and a prefix evaluated once for a by-name default
// read twice (`Lifter.liftPrefix`).
class Outer[T](val t: T, val n: Int) {
  private val secret = 7
  var count = 0
  class ByName(y: => Int = { count += 1; n }) { def x = y }
  case class Later(a: Int)(b: => Int = { count += 10; a + n }) { def c = b }
  object Later
  case class Fn(f: Int => Int = _ + n)
  case class Priv(x: Int = secret)
  case class Gen(x: T = t, xs: List[T] = List(t, t))
  class Mid(val m: Int) {
    case class Deep(x: Int = m + n)
    object Deep
  }
  class Plain(val b: Int = n * 2)
  class Sub extends Plain
  case class Quiet(a: Int = n + 1)
  object Quiet { def unused = 0 }
  case class Lazy(a: Int)(b: => Int = n) { def c = b }
}

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer("q", 5)
    val b = new o.ByName()
    println(s"${b.x} ${b.x} ${o.count}")
    val l = o.Later(1)()
    println(s"${l.c} ${l.c} ${o.count}")
    println(o.Fn().f(1))
    println(o.Priv().x)
    println(o.Gen())
    val m = new o.Mid(100)
    println(m.Deep().x)
    println(new o.Sub().b)
    println(new o.Quiet().a)
    def recv = { println("receiver"); o }
    val z = recv.Lazy(1)()
    println(z.c + z.c)
  }
}
