// An object or class named by an operator, its body in braces on the header's line (`object +++ { .. }`, where a `{`
// after an operator is otherwise a block argument) or after a colon.
class P(val a: Int, val b: Int)
object +++ { def unapply(p: P): (Int, Int) = (p.a, p.b) }
object --- :
  def unapply(p: P): (Int, Int) = (p.b, p.a)
class ::: { override def toString = "colons" }
object Main:
  def main(args: Array[String]): Unit =
    P(1, 2) match
      case x +++ y => println(x * 10 + y)
    P(1, 2) match
      case x --- y => println(x * 10 + y)
    println(new :::)
