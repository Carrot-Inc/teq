// A `return` of the def's call in a local lazy val's initialiser is no loop's jump: the initialiser runs in a method
// of its own (dotty's TailRec takes its right-hand side as no tail position), so the call stays a call. On
// JavaScript the `return` leaves the initialiser's function, not the def (teq's row: `9`).
object Z {
  final def alone(n: Int): Int = { lazy val x: Int = { if n > 0 then return alone(n - 1); 5 }; x + 1 }
  final def counted(n: Int, acc: Int): Int = { lazy val y: Int = { if n > 0 then return counted(n - 1, acc + 1); acc }; y * 2 }
}
object Main {
  def main(args: Array[String]): Unit = {
    println("start")
    println(Z.alone(3))
    println(Z.counted(4, 0))
  }
}
