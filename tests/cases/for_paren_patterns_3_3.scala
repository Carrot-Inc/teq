// A pattern in parentheses is dotty's `Parens` (`Desugar.makeFor`'s `isVarBinding` does not unwrap it, `isIrrefutable`
// unwraps it to a variable or `_` alone), so before 3.4 a typed pattern, a stable identifier or a backquoted name in
// parentheses is filtered: `withFilter`, then `map`. A variable in parentheses, a typed variable and `given` are not.
// teq: --source 3.3
class Box[A](val a: A) {
  def map[B](f: A => B): Box[B] = { println("map"); new Box(f(a)) }
  def withFilter(f: A => Boolean): Box[A] = { println("filter"); if !f(a) then throw new Exception("filtered"); this }
}
object X
object P {
  val x = 1
  def main(args: Array[String]): Unit = {
    println("(y: Int)"); println((for { (y: Int) <- new Box(1) } yield y + 1).a)
    println("((y: Int))"); println((for { ((y: Int)) <- new Box(2) } yield y + 1).a)
    println("(_: Int)"); println((for { (_: Int) <- new Box(3) } yield 0).a)
    println("(y)"); println((for { (y) <- new Box(4) } yield y + 1).a)
    println("y: Int"); println((for { y: Int <- new Box(5) } yield y + 1).a)
    println("_: Int"); println((for { _: Int <- new Box(6) } yield 0).a)
    println("(X)"); println((for { (X) <- new Box(X) } yield 7).a)
    println("X"); println((for { X <- new Box(X) } yield 8).a)
    println("(`x`)"); println((for { (`x`) <- new Box[x.type](x) } yield 9).a)
    println("(y): Int"); println((for { (y): Int <- new Box(10) } yield y + 1).a)
    println("((y, z))"); println((for { ((y, z)) <- new Box((11, 1)) } yield y + z).a)
    println("(y @ (z: Int))"); println((for { (y @ (z: Int)) <- new Box(12) } yield y + z).a)
    println("given Int"); println((for { given Int <- new Box(13) } yield summon[Int]).a)
  }
}
