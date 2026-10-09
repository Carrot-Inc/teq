// Implicit conversions of both kinds and both uses; every expectation is scalac 3.8.4's.
import scala.language.implicitConversions
import scala.annotation.nowarn

trait Eq[A] { def eqv(a: A, b: A): Boolean }
object Eq {
  implicit val eqInt: Eq[Int] = (a, b) => a == b
  implicit val eqStr: Eq[String] = (a, b) => a == b
}
final class EqOps[A](val lhs: A)(implicit ev: Eq[A]) {
  def ===(rhs: A): Boolean = ev.eqv(lhs, rhs)
  def =!=(rhs: A): Boolean = !ev.eqv(lhs, rhs)
}
trait Semigroup[A] { def combine(a: A, b: A): A }
object Semigroup {
  implicit val sgInt: Semigroup[Int] = (a, b) => a + b
  implicit def sgList[A]: Semigroup[List[A]] = (a, b) => a ++ b
}
class SemigroupOps[A](lhs: A)(implicit ev: Semigroup[A]) {
  def |+|(rhs: A): A = ev.combine(lhs, rhs)
}
object syntax {
  implicit def toEqOps[A](a: A)(implicit ev: Eq[A]): EqOps[A] = new EqOps[A](a)
  implicit def toSemigroupOps[A](a: A)(implicit ev: Semigroup[A]): SemigroupOps[A] = new SemigroupOps[A](a)
}

class Meters(val n: Int) {
  def show: String = s"${n}m"
  def plus(o: Meters): Meters = new Meters(n + o.n)
}
object Meters {
  implicit def fromInt(i: Int): Meters = new Meters(i)
}
class Celsius(val deg: Int) { def describe: String = s"${deg}C" }
object Celsius {
  implicit def fromMeters(m: Meters): Celsius = new Celsius(m.n)
}
class Box(val items: List[Int]) {
  def apply(i: Int): Int = items(i)
  def update(i: Int, v: Int): Unit = println(s"update $i $v")
  var field: Int = 0
  val howMany: Int = items.size
  def total: Int = items.sum
}
object Box {
  implicit def fromList(xs: List[Int]): Box = new Box(xs)
}

// Extension versus conversion: a member wins, then a lexical extension, then a given's
// extension, then a conversion.
class Word(val s: String) { def shout: String = s.toUpperCase }
object WordConv {
  implicit def toWord(s: String): Word = new Word(s)
}
object WordExt {
  extension (s: String) def shout: String = s"ext:$s"
}
trait Shouter[A] { extension (a: A) def shout: String }
object ShouterInstances {
  given Shouter[String] with { extension (s: String) def shout: String = s"given:$s" }
}

// Two conversions providing the same member: the one taking the more specific type wins.
class Animal(val name: String)
class Dog(name: String) extends Animal(name)
class Described(val text: String) { def describe: String = text }
object Describe {
  implicit def animalDescribed(a: Animal): Described = new Described(s"animal ${a.name}")
  implicit def dogDescribed(d: Dog): Described = new Described(s"dog ${d.name}")
}

// A conversion whose implicit parameter has no instance is passed over for the next one.
trait Marker[A]
object Marker { implicit val markerInt: Marker[Int] = new Marker[Int] {} }
class Tagged(val tag: String) { def tagged: String = tag }
object Tags {
  implicit def withMarker[A](a: A)(implicit m: Marker[A]): Tagged = new Tagged("marked")
  implicit def plain(s: String): Tagged = new Tagged("plain")
}

// A conversion to a type parameter's instantiation, and a `Conversion` given.
class Wrapper[A](val a: A) { def unwrap: A = a }
object Wrappers {
  implicit def wrap[A](a: A): Wrapper[A] = new Wrapper(a)
}
class Pounds(val p: Int) { def lbs: String = s"${p}lbs" }
object Pounds {
  given Conversion[Int, Pounds] = i => new Pounds(i)
  given Conversion[Meters, Pounds] with { def apply(m: Meters): Pounds = new Pounds(m.n * 2) }
}

// An implicit class and a Scala 2 implicit competing with a given.
object RichThings {
  implicit class RichInt(private val n: Int) {
    def squared: Int = n * n
    def times(k: Int): Int = n * k
  }
  implicit class Usd(n: Double) {
    def usd: String = s"${n + 0.5} usd"
  }
  implicit class Ticks(n: Int) {
    def ticks: String = s"$n int ticks"
  }
  implicit class LongTicks(n: Long) {
    def ticks: String = s"$n long ticks"
  }
  implicit class RichList[A](xs: List[A]) {
    def second: A = xs(1)
  }
}
class Label(val text: String) { def label: String = text }
object LabelConv {
  implicit def fromInt(i: Int): Label = new Label(s"implicit $i")
}
object LabelGiven {
  given Conversion[Int, Label] = i => new Label(s"given $i")
}

object Test {
  import syntax._
  import Meters.fromInt
  def takesMeters(m: Meters): String = m.show
  def overloaded(m: Meters): String = "meters " + m.show
  def overloaded(s: String): String = "string " + s
  def bothRounds(c: Celsius): String = "celsius " + c.describe
  def bothRounds(xs: List[Int]): String = "list " + xs.size

  def precedence(): Unit = {
    import WordConv._
    println("a".shout)
    def lexical(): Unit = {
      import WordExt._
      println("b".shout)
    }
    lexical()
    def viaGiven(): Unit = {
      import ShouterInstances.given
      println("c".shout)
    }
    viaGiven()
    println(new Word("d").shout)
  }

  def specific(): Unit = {
    import Describe._
    println(new Dog("rex").describe)
    println(new Animal("cat").describe)
    val a: Animal = new Dog("fido")
    println(a.describe)
  }

  def fallthrough(): Unit = {
    import Tags._
    println(1.tagged)
    println("s".tagged)
  }

  def members(): Unit = {
    import Box._
    val xs = List(10, 20, 30)
    println(xs.howMany + xs.total)
    xs(0) = 5
    xs.field = 3
    val b: Box = xs
    println(b(2))
  }

  def generic(): Unit = {
    import Wrappers._
    println(3.unwrap + "x".unwrap)
    val w: Wrapper[String] = "y"
    println(w.unwrap)
  }

  def givens(): Unit = {
    import Pounds.given
    println(5.lbs)
    println(new Meters(4).lbs)
    val p: Pounds = 7
    println(p.lbs)
    val q: Pounds = new Meters(1)
    println(q.lbs)
  }

  def implicitClass(): Unit = {
    import RichThings._
    println(4.squared + 3.times(5))
    println(List("p", "q").second)
    println(44.usd)
    println(2.25.usd)
    println(44L.usd)
    val n: 44 = 44
    val c: 'a' = 'a'
    println(n.usd)
    println(c.usd)
    println((-1).usd)
    println(3.ticks)
    println(3L.ticks)
  }

  def competing(): Unit = {
    import LabelConv._
    println(1.label)
    def inner(): Unit = {
      import LabelGiven.given
      println(2.label)
    }
    inner()
  }

  def adaptation(): Unit = {
    val m: Meters = 5
    println(m.show)
    println(takesMeters(3))
    println((7: Meters).show)
    println(3.plus(new Meters(2)).show)
    val c: Celsius = new Meters(9)
    println(c.describe)
    def ret(): Meters = 8
    println(ret().show)
    val f: Int => Meters = i => i
    println(f(2).show)
    val branch: Meters = if (m.n > 1) 1 else new Meters(2)
    println(branch.show)
  }

  def overloads(): Unit = {
    println(overloaded(4))
    println(overloaded("x"))
    println(bothRounds(new Meters(6)))
    println(bothRounds(List(1, 2)))
  }

  def main(args: Array[String]): Unit = {
    println(1 === 1)
    println("a" =!= "b")
    println((1 |+| 2) + " " + (List(1) |+| List(2)))
    precedence()
    specific()
    fallthrough()
    members()
    generic()
    givens()
    implicitClass()
    competing()
    adaptation()
    overloads()
  }
}
