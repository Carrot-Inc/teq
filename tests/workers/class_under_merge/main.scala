// Publishing `U.make`'s body settles its anonymous class's `put`, whose comparison completes
// `Left.put`'s inferred signature and publishes its body inside the merge. That publication leaves
// the class alone, `get` as well: `get`'s comparison would complete `Left.get`, which reads the
// `Left.put` still being completed, a recursive signature to the parallel path.
// scalac prints 42.
// Without `Dummy0` the entry-wise rule gave way in none of eight runs.
class Dummy0
abstract class Root:
  def put(x: Int): Unit
  def get(x: Int): Unit
class Basic extends Root:
  def put(x: Int): Unit = ()
  def get(x: Int): Unit = ()
object U:
  def make(): Basic = new Basic with Left
trait Left extends Root:
  abstract override def put(x: Int) = super.put(x)
  abstract override def get(x: Int) = put(x)
@main def run(): Unit =
  U.make().get(1)
  println(42)
