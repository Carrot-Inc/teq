import scala.annotation.nowarn

object Ret {
  @nowarn def find(xs: List[Int], p: Int => Boolean): Option[Int] = {
    for (x <- xs) if (p(x)) return Some(x)
    None
  }
  @nowarn("msg=Non local") def firstNeg(xs: List[Int]): Int = {
    xs.foreach { x =>
      if (x < 0) return x
    }
    0
  }
  def early(n: Int): String = {
    if (n < 0) return "neg"
    if (n == 0) {
      return "zero"
    }
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
  @nowarn
  def nested(xss: List[List[Int]]): Int = {
    xss.foreach { xs =>
      xs.foreach { x =>
        if (x == 7) return x * 10
      }
    }
    -1
  }
  def withLocal(n: Int): Int = {
    def inner(k: Int): Int = {
      if (k > 2) return 99
      k
    }
    if (n < 0) return -100
    inner(n) + 1
  }
  def afterReturn(n: Int): Int = {
    if (n > 0) return n
    else return -n
  }
  def stored(): Int => Int = {
    val f: Int => Int = x => x + 1
    f
  }
  def returnInArg(n: Int): Int = {
    Math.max(n, if (n > 100) return 0 else 1)
  }
  def main(args: Array[String]): Unit = {
    println(find(List(1, 2, 3), _ > 1))
    println(find(List(1, 2, 3), _ > 5))
    println(firstNeg(List(1, -2, -3)))
    println(firstNeg(List(1, 2)))
    println(early(-1) + early(0) + early(2) + early(5))
    unitReturn(1); unitReturn(3)
    println(loop(4, 0))
    println(inMatch(Some(20)) + inMatch(Some(2)) + inMatch(None))
    println(inVal(3) + inVal(9))
    println(nested(List(List(1), List(7, 8))) + nested(List(Nil)))
    println(withLocal(5) + withLocal(1) + withLocal(-3))
    println(afterReturn(3) + afterReturn(-4))
    println(stored()(1))
    println(returnInArg(5) + returnInArg(500))
  }
}
