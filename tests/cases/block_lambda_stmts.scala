// A lambda at the start of a block takes the rest of the block as its body, semicolons included.
object Main:
  var log = List.empty[String]
  def main(args: Array[String]): Unit =
    println(List(1, 2).map { i => log = s"m$i" :: log; i * 10 })
    val f: Int => Int = { i => log = s"f$i" :: log; i + 1 }
    println(f(3))
    val g = (a: Int, b: Int) => { log = "g" :: log; a * b }
    println(g(2, 3))
    println(List(4).map { (i: Int) => val d = i * 2; d + 1 })
    println(Option(5).map { i =>
      log = "o" :: log
      i - 1
    })
    println(log.reverse)
