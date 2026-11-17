// The ClassTag of a class named through a path is its class's (`Synthesizer.synthesizedClassTag`: the erasure of
// `p.C` is `C`): through a value, a value import, a parameter, the object that inherits the class's trait, a class
// directly in a class, the class's own inner object named inside the class and from a class nested in it, a
// subclass's `this`, a type's projection, a local class's instance; as the tag's subject and inside a union, an
// intersection, an array and a nullable; two instances' tags are equal; `new Array[p.C](n)` holds the class's values.
import scala.reflect.ClassTag

class Ops(val seed: Int):
  object nums:
    trait Sh { def name: String }
    class A extends Sh { def name = "a" }
    class B extends Sh { def name = "b" }
    trait Mark
    class Box(val v: Int) { def sum: Int = v + seed }
    case class Pt(x: Int)
    class Cell[T](val get: T)
    enum Color { case Red, Green }
    type Alias = Box
  class Top(val v: Int)
  def inside(): String =
    val a = Array.empty[nums.Box]
    val b = Array(nums.Pt(1), nums.Pt(2))
    s"${a.length} ${b.map(_.x).sum} ${summon[ClassTag[this.nums.Box]].runtimeClass.getSimpleName.nonEmpty}"
  class Inner:
    def nested(): Int = Array.fill(2)(new nums.Box(1)).map(_.sum).sum

trait Kit:
  def seed: Int
  class Box(val v: Int) { def sum: Int = v + seed }
  case class Pt(x: Int)
object tools extends Kit { def seed: Int = 2 }

class Sub extends Ops(5):
  def own(): Int = Array.empty[Top].length + Array(new Top(1)).length

def make[T: ClassTag](n: Int): Array[T] = new Array[T](n)
def viaParam(o: Ops): Int =
  import o.nums.*
  Array.ofDim[Box](3).length + List(Pt(1)).toArray.length

@main def run(): Unit =
  val o = new Ops(2)
  val p = new Ops(10)
  println(Array.empty[o.nums.Sh].length)
  println(summon[ClassTag[o.nums.Box]] == summon[ClassTag[p.nums.Box]])
  println(Array.empty[o.nums.Box].getClass == Array.empty[p.nums.Box].getClass)
  println(make[o.nums.Box](2).length)
  println(Array(new o.nums.Cell(1)).length + Array.empty[o.nums.Cell[Int]].length)
  println(Array(o.nums.Color.Red, o.nums.Color.Green).length)
  println(Array.empty[o.nums.Alias].length)
  locally:
    import o.nums.*
    val xs = Array[A | B](new A, new B)
    println(xs.map(_.name).mkString)
    println(Array[Box | Null](new Box(1), null).length)
    println(Array[Box & Mark]().length)
    val grid = Array.ofDim[Box](2, 2)
    grid(1)(0) = new Box(3)
    println(grid(1)(0).sum + Array(Array(new Box(1))).length)
    println(IArray(Pt(1)).length)
  println(viaParam(o))
  import tools.*
  println(Array(new Box(1), new Box(2)).map(_.sum).sum + Array.empty[tools.Pt].length)
  println(o.inside())
  println(new o.Inner().nested())
  println(new Sub().own())
  println(Array[Ops#Top](new o.Top(1), new p.Top(2)).map(_.v).sum + Array.empty[Ops#Top].length)
  val made = new Array[o.nums.Box](2)
  made(1) = new o.nums.Box(7)
  println(made(1).sum + made.length)
  val both = new Array[o.nums.A | o.nums.B](2)
  both(0) = new o.nums.B
  println(both(0).name)
  class Scope:
    class Leaf(val v: Int)
  val s = new Scope
  println(Array(new s.Leaf(1), new s.Leaf(2)).map(_.v).sum + Array.empty[s.Leaf].length)
