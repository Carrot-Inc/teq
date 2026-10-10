// A class nested in a class has a constructor proxy, a member of the class that declares it
// (dotty's `NamerOps.addConstructorProxies`): `o.Inner(1)` is `new o.Inner(1)`, an import of a
// value's members brings it and its class with `o` as their prefix (renamed too), and a class
// deriving from the declaring one, an object included, has it as an inherited member.
class O(val n: Int):
  class Inner(val x: Int):
    def get = n + x
  class Pair[A](val a: A, val b: A):
    def show = s"$n:$a,$b"
trait T:
  val base: Int
  class Box(val v: Int):
    def total = base + v
object Ob extends T:
  val base = 100
  def make(x: Int) = Box(x)
  def made = new Box(2)
class C2 extends T:
  val base = 200
  def make(x: Int) = Box(x)

class Holder:
  case class Foo(x: Int, y: Int)
sealed trait Schema[A]
case class Field[A]()
object Schema extends RecordInstances
sealed trait RecordInstances:
  case class Record[A](field: Field[A]) extends Schema[A]

def make(o: O): o.Inner =
  import o.*
  new Inner(1)

@main def run(): Unit =
  val o = new O(10)
  println(new o.Inner(1).get)
  println(o.Inner(2).get)
  val f: Int => o.Inner = o.Inner(_)
  println(f(3).get)
  println(o.Pair(1, 2).show)
  println(o.Pair[String]("a", "b").show)
  locally {
    import o.*
    println(new Inner(3).get)
    println(Inner(4).get)
    println(Pair(true, false).show)
  }
  val o2 = new O(20)
  import o2.{Inner => Renamed}
  println(new Renamed(5).get)
  println(Renamed(6).get)
  println(make(new O(30)).get)
  println(Ob.make(1).total)
  println(Ob.made.total)
  println(Ob.Box(3).total)
  val t: T = Ob
  println(t.Box(4).total)
  println((new C2).make(5).total)
  locally {
    import Ob.*
    println(Box(5).total)
  }
  locally {
    import Schema.*
    val field: Field[Int] = Field()
    println(Record[Int](field))
  }
  val h = new Holder
  val foo = h.Foo(1, 2)
  println(foo)
  foo match
    case h.Foo(x, y) => println(x + y)
