// A right-associative operator evaluates its left operand before the object on its right is loaded, the object's
// loading showing through a trait parameter's default: `effect("l") :: Kept` binds the left operand first.
// The expectation is scalac 3.8.4's.
trait Shown(x: Int = { println("  Kept loaded"); 1 })
object Kept extends Shown {
  def ::(x: Any): Unit = println("  :: called")
}
object Main {
  def effect(tag: String): Int = { println("  effect " + tag); 1 }
  def main(args: Array[String]): Unit = {
    println("a right-associative operator: its left operand, then the object")
    effect("l") :: Kept
    println("end")
  }
}
