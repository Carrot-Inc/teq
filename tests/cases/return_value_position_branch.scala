// A `return` where a conditional branch takes its value: a match guard's `&&` whose right operand returns a call
// of the def by itself, an `if`'s condition, `&&`'s and `||`'s right operands, `!`'s operand, a `while`'s
// condition. Nothing is written after the `return`; the dead branch that follows verifies.
object R {
  final def guardB(n: Int): Boolean = n match {
    case k if k > 0 && { return (k > -1 && guardB(k - 1)) } => false
    case _ => true
  }
  def condRet(n: Int): Int = if ({ return n + 1 }) then 3 else 4
  def andRet(n: Int): Int = if (n > 0 && { return 2 }) then 3 else 4
  def notRet(n: Int): Boolean = !({ return n > 0 }: Boolean)
  def whileCond(n: Int): Int = { while ({ return n * 2 }) do (); 0 }
  def orRet(n: Int): Boolean = n < 0 || { return n == 5 }
}

object Main {
  def main(args: Array[String]): Unit = {
    println(s"${R.guardB(3)} ${R.guardB(1000)}")
    println(s"${R.condRet(4)} ${R.andRet(1)} ${R.andRet(0)}")
    println(s"${R.notRet(2)} ${R.whileCond(3)}")
    println(s"${R.orRet(-1)} ${R.orRet(5)} ${R.orRet(6)}")
  }
}
