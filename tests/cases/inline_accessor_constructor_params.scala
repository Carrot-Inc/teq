// An inline method's reads of the class's private constructor parameters, a plain one and a `private val`: each is
// read through the accessor `PrepareInlineable` makes (`inline$Strict$$k`), which the class's body gets
// (`addAccessorDefs`), so every target emits it.
class Strict(k: Int):
  inline def twice: Int = k * 2
class Held(private val k: Int, m: Int):
  inline def sum: Int = k + m
  inline def scaled(f: Int): Int = (k + m) * f
object M:
  def main(args: Array[String]): Unit =
    println(new Strict(21).twice)
    val h = new Held(1, 2)
    println(h.sum)
    println(h.scaled(5))
