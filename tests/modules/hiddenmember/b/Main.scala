package hiddenmember

object Main:
  def main(args: Array[String]): Unit =
    println(C(1))
    println(C(1).productPrefix)
    println((C(2): Product).productPrefix)
    println(D(3))
    println(s"${D(4)} ${C(5)}")
