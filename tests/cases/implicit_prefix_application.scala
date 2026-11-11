trait Evidence
object PrefixOps:
  extension (n: => Int)(using Evidence)
    def repeat(suffix: String): String = s"$n,$n$suffix"
  def defaulted(using n: Int = 7)(suffix: String): String = s"$n$suffix"
  def leading(using Evidence)(n: Int)(suffix: String): String = s"$n$suffix"

@main def run(): Unit =
  import PrefixOps.*
  given Evidence = new Evidence {}
  var reads = 0
  def next(): Int = { reads += 1; reads }
  println(next().repeat("!"))
  println(reads)
  println(defaulted("?"))
  println(leading(5)("."))
