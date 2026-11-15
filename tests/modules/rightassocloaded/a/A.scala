// Receivers of a right-associative operator that another module's products give: a top-level value, an object's
// value, an object with a value, an object whose parent's constructor takes an argument with an effect. Each
// runs an initializer at its first read, so the operand is bound ahead of it, over the products as in the whole
// build: an object read from products is not taken to be without an initializer on its record's word.
package other
val items: List[Int] = { println("init items"); List(1) }
object Holder:
  println("init holder")
  val held: List[Int] = List(2)
object Quiet:
  val held: List[Int] = { println("init quiet"); List(3) }
object Op:
  val x: Int = { println("init op"); 4 }
  def +:(n: Int): String = "op" + n
class Base(val n: Int)
object Param extends Base({ println("init param"); 5 }):
  def +:(k: Int): String = "param" + k
