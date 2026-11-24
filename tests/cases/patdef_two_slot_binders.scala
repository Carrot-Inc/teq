// A pattern definition whose one variable is a long or a double takes two slots before its right side is evaluated, as
// scalac's does: an `if` on the variable after it puts a frame at a branch target, which failed verification while the
// definition took one slot (the right side's scrutinee then shared the second).
object Main {
  def long(): Unit = {
    val Some(x) = Some(1L)
    if (x > 0) println(x)
  }

  def double(): Unit = {
    val Some(x) = Some(1.5)
    if (x > 0) println(x)
  }

  def amongLocals(): Unit = {
    val i = 1
    val l = 2L
    val s = "s"
    val Some(x) = Some(3L)
    val d = 4.5
    if (x > l) println(s"$i $l $s $x $d") else println("no")
  }

  def inTry(pair: (Int, Long)): Unit = {
    try {
      val Some(x) = Some(2.5)
      val (a, b) = pair
      if (x > 1 && b > a) println(s"$x $a $b")
    } finally println("finally")
  }

  def inLoop(pair: (Long, Int)): Unit = {
    var k = 0
    while (k < 2) {
      val Some(x) = Some(k * 10L)
      val Some((a, b)) = Some(pair)
      if (x >= 0 && a > b) println(s"$x $a $b")
      k += 1
    }
  }

  def aliases(): Unit = {
    for { k <- List(1, 2); _ = 5L } println(k)
    for { k <- List(1, 2); Some(x) = Some(2.5) } if (x > 0) println(s"$k $x")
  }

  def main(args: Array[String]): Unit = {
    long()
    double()
    amongLocals()
    inTry((1, 2L))
    inLoop((3L, 2))
    aliases()
  }
}
