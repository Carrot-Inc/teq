trait Reader:
  def read(): Int
object Main:
  inline def twice(r: Reader): Int = r.read() + r.read()
  transparent inline def twiceT(r: Reader): Int = r.read() + r.read()
  def main(args: Array[String]): Unit =
    val k = 7
    println(twice({ class D(val m: Int) { class Inner { def get = m + k }; def rd: Reader = new Reader { def read() = new Inner().get } }; new D(5).rd }))
