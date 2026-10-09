// Publishing `U.make`'s body settles its anonymous class's `put`, whose comparison completes
// `Left.put`'s inferred signature; that types `Left.put`'s body, which makes a second anonymous
// class, and publishes the body inside the merge. The second class is settled by that publication
// before its seal reaches the class.
// scalac prints 42.
abstract class Root:
  def put(x: Int): Unit
class Basic extends Root:
  def put(x: Int): Unit = ()
object U:
  def make(): Basic = new Basic with Left with Right
trait Left extends Root:
  abstract override def put(x: Int) =
    val b = new Basic with InnerLeft with InnerRight
    super.put(x)
trait Right extends Root:
  abstract override def put(x: Int): Unit = super.put(x)
trait InnerLeft extends Root:
  abstract override def put(x: Int): Unit = super.put(x)
trait InnerRight extends Root:
  abstract override def put(x: Int): Unit = super.put(x)
@main def run(): Unit =
  U.make().put(1)
  println(42)
