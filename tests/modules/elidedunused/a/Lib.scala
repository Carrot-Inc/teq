package ela

// Bodies the TASTy writer withholds (`IndexedSeq.range`, a shape of the lean std scala-library has
// otherwise): a downstream that needs one fails naming it; one that never
// reaches it builds and runs.
object Calls:
  def range: Int = IndexedSeq.range(0, 3).length
  def plain: Int = 7

object Init:
  val withheld = { print("init "); IndexedSeq.range(0, 1) }
  def f = 0

class Base(val n: Int)
class Parent extends Base(IndexedSeq.range(0, 2).length)

class Stmt:
  println(IndexedSeq.range(0, 1).toList)
  def k = 1
