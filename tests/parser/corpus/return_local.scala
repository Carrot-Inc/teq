object Ret2 {
  def early(n: Int): String = {
    if (n < 0) return "neg"
    var i = 0
    while (i < n) {
      if (i == 3) return s"hit $i"
      i += 1
    }
    "pos"
  }
  def unitReturn(n: Int): Unit = {
    if (n > 2) return
    println(s"small $n")
  }
  def loop(n: Int, acc: Int): Int = {
    if (n == 0) return acc
    loop(n - 1, acc + n)
  }
  def inMatch(o: Option[Int]): Int = o match {
    case Some(v) if v > 10 => return 100
    case Some(v) => v
    case None => return -1
  }
  def inVal(n: Int): Int = {
    val x = if (n > 5) return 5 else n
    x * 2
  }
  def returnInArg(n: Int): Int = Math.max(n, if (n > 100) return 0 else 1)
  def longRet(n: Int): Long = { if (n > 0) return 5L; 6L }
  def main(args: Array[String]): Unit = {
    println(early(-1) + early(2) + early(5))
    unitReturn(1); unitReturn(3)
    println(loop(4, 0))
    println(inMatch(Some(20)) + inMatch(Some(2)) + inMatch(None))
    println(inVal(3) + inVal(9))
    println(returnInArg(5) + returnInArg(500))
    println(longRet(1) + longRet(-1))
  }
}
