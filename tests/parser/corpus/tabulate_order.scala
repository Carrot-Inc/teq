// The function of tabulate is called from 0 up, as an effect in it shows.
object Main:
  def main(args: Array[String]): Unit =
    var calls = List.empty[Int]
    val xs = List.tabulate(4) { i => calls = i :: calls; i * 10 }
    println(xs)
    println(calls.reverse)
    calls = Nil
    val vs = Vector.tabulate(3) { i => calls = i :: calls; i + 1 }
    println(vs)
    println(calls.reverse)
    calls = Nil
    val as = Array.tabulate(3) { i => calls = i :: calls; i * i }
    println(as.mkString(","))
    println(calls.reverse)
    println(Seq.tabulate(2)(_ * 2))
    println(List.tabulate(0)(identity))
