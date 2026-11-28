// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s
object Main:
  def search(n: Int): Int =
    try
      val visit = (x: Int) =>
        if x > 0 then return x
        println("continued")
      visit(n)
      -1
    finally println("search cleanup")

  def main(args: Array[String]): Unit =
    println(search(8))
    println(search(0))
