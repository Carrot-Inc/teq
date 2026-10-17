package tuplematchpatdef

object Main:
  def main(args: Array[String]): Unit =
    val (w, f) = Amounts.split("12.50")
    val (m, n) = Amounts.sign(-7)
    println(s"$w $f $m $n ${Amounts.unit} ${Amounts.scale}")
    val (_, rest) = Amounts.split("3")
    println(s"[$rest]")
    println(Amounts.place(true) ++ Amounts.place(false))
