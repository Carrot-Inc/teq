trait Reader:
  def read(): Int
object Main:
  inline def twice(r: Reader): Int = r.read() + r.read()
  transparent inline def twiceT(r: Reader): Int = r.read() + r.read()
  def main(args: Array[String]): Unit =
    val k = 7
    println(twice({ val d = 4; def f = d + 1; class E extends Reader { def read() = f }; new E }))
