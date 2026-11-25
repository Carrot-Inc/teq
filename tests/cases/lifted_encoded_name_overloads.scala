// Local definitions named `+` and `$plus` in two methods of one class are lifted under one encoded name, `$plus$1`:
// dotty's counter numbers them alike, and their descriptors tell the two methods apart, as overloads. The same holds
// for two local lazy vals, whose accessors and initialisers share `$plus$1` and `$plus$lzyINIT1$1`.
object Test:
  def a(): Int =
    def +(i: Int): Int = { println("int"); i + 1 }
    `+`(2)
  def b(): String =
    def $plus(s: String): String = { println("string"); s + "!" }
    $plus("x")
  def c(): Int =
    lazy val + = { println("lazy int"); 3 }
    `+`
  def d(): String =
    lazy val $plus = { println("lazy string"); "y" }
    $plus
  def main(args: Array[String]): Unit =
    println(a())
    println(b())
    println(c())
    println(d())
