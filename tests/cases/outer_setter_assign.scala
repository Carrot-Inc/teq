// An unqualified assignment to a def and its setter `x_=` of an enclosing class calls the setter on that class's
// `this`, the prefix the getter is read through (dotty's `Typer.typedAssign`); master refused it ("this expression
// cannot be assigned to") where the pair is not the current class's own. A compound assignment takes the same
// path, two classes out too, an imported object's pair as well, and an enclosing class's var is assigned as before.
class Outer:
  private var v = 0
  var plain = 0
  def x: Int = v
  def x_=(n: Int): Unit = { v = n * 10 }
  class Inner:
    def set(): Unit = { x = 2 }
    def add(): Unit = { x += 1 }
    def setPlain(): Unit = { plain = 7 }
    class Deeper:
      def set(): Unit = { x = 3 }
  def test(): String =
    val in = new Inner
    in.set()
    val a = x
    in.add()
    val b = x
    new in.Deeper().set()
    in.setPlain()
    s"$a $b $x $plain"
object Holder:
  private var h = 0
  def y: Int = h
  def y_=(n: Int): Unit = { h = n + 1 }
@main def run() =
  println(new Outer().test())
  import Holder.*
  y = 4
  y += 1
  println(Holder.y)
