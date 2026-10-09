// foldLeft(Nil) with an untyped lambda: the improved accumulator's element variable is settled by the body, not by the first selection on it
//> using scala 3.8.4
object Main:
  def fl[Z](z: Z)(op: (Z, Int) => Z): Z = op(z, 1)
  def main(args: Array[String]): Unit =
    val xs = List(3, 1, 2)
    println(xs.foldLeft(Nil)((acc, x) => x :: acc))
    println(List(3, 1, 2).foldLeft(Nil)((acc, x) => x :: acc))
    println(xs.map(_ + 1).foldLeft(Nil)((acc, x) => x :: acc))
    println(xs.foldLeft(Nil)((acc, x) => acc.::(x)))
    println(xs.foldLeft(Nil)((acc, x) => { val r = x :: acc; r }))
    println(fl(Nil)((acc, x) => x :: acc))
    println(fl(Nil)((acc, x) => { val r: List[Int] = x :: acc; r }))
    val ys: List[Int] = xs.foldLeft(Nil)((acc, x) => x :: acc)
    println(ys.sum)
