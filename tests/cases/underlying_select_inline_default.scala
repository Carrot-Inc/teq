class V(val i: Int) extends AnyVal

object Main:
  def make(i: Int): V =
    println("make " + i)
    new V(i)

  inline def inlined(i: Int): V = make(i)

  def read(v: V = make(9)): Int =
    v
      .i

  def main(args: Array[String]): Unit =
    println(inlined(7)
      .i)
    println(read())
