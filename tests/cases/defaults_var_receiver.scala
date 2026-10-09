// A var's read is evaluated where the source has it: a receiver of a call with defaults and a
// named argument read before a later argument assigns the var.
class DefaultsVarBox(val n: Int) {
  def f(a: Int, b: Int = n): Int = b
}
object DefaultsVar {
  def g(a: Int = 0, b: Int, c: Int): Int = b
  def receiver(): Int = {
    var c = new DefaultsVarBox(1)
    c.f({ c = new DefaultsVarBox(2); 0 })
  }
  def argument(): Int = {
    var x = 1
    g(b = x, c = { x = 2; 0 })
  }
  def main(args: Array[String]): Unit = {
    println(receiver())
    println(argument())
  }
}
