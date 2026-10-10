package nestedhidden

object Main:
  def main(args: Array[String]): Unit =
    println(Scope.Item(3))
    println((Scope.Item(4): Product).productPrefix)
