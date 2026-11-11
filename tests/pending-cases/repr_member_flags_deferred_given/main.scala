trait U[-A, +B] { def apply(using x: A): B; def f(y: Int): Int }
object Main {
  def main(args: Array[String]): Unit = {
    println(M.inspect[U[Int, Int]])
    println(M.inspect[scala.ContextFunction1[Int, Int]])
  }
}
