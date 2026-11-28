object Main {
  def mix(i: Int, l: Long, d: Double, n: Int): Int = {
    println(s"$i $l $d $n")
    if n < 4 then mix(n + l.toInt, i.toLong - d.toLong, l.toDouble + n + 0.5, i + d.toInt)
    else i + l.toInt + d.toInt
  }
  def main(args: Array[String]): Unit = println(mix(4, 7L, 5.5, 2))
}
