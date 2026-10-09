// A lambda that is no `{ case ... }` given to a method overloaded on a function and a partial
// function goes to the function, total: dotc's first pass of overloading resolution leaves out
// the SAM conversion of a function literal to a partial function (`resolveOverloaded`,
// `argOK`), which a `{ case ... }` literal does not need; where only partial functions are
// taken, the second pass takes them. The expected result still picks the partial function's
// alternative where the function's result does not conform (`adaptByResult`), one that takes the
// literal's parameters as a pair among them.
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape

object Over:
  def run(f: Int => Int): String = "fn"
  def run(pf: PartialFunction[Int, Int]): String = "pf"
  def shape(f: Shape => Int): String = "fn " + f(Circle(1))
  def shape(pf: PartialFunction[Shape, Int]): String = "pf " + pf.isDefinedAt(Square(1))
  def pair(f: Int => Int, n: Int): String = "fn"
  def pair(pf: PartialFunction[Int, Int], s: String): String = "pf"
  def rest(f: Int => Int): String = "fn"
  def rest(pf: PartialFunction[Int, Int], more: Int*): String = "pf"
  def only(pf: PartialFunction[Shape, Int]): String = "pf " + pf.isDefinedAt(Square(1))
  def only(pf: PartialFunction[Shape, String], tag: String): String = "pf " + tag + pf.isDefinedAt(Square(1))
  def choose(f: Int => Int): Int = 1
  def choose(pf: PartialFunction[Int, Int]): String = "pf " + pf.isDefinedAt(3)
  def pairs(g: (Int, Int) => Int): String = "fn2 " + g(3, 4)
  def pairs(pf: PartialFunction[(Int, Int), Int]): Int = pf((3, 4)) * 10

class Built(val tag: String):
  def this(f: Shape => Int) = this("fn " + f(Circle(1)))
  def this(pf: PartialFunction[Shape, Int], d: DummyImplicit) = this("pf " + pf.isDefinedAt(Square(1)))

object Main:
  def main(args: Array[String]): Unit =
    println(Over.run(x => x match { case 1 => 2 }))
    println(Over.run((x: Int) => x match { case 1 => 2 }))
    println(Over.run { case 1 => 2 })
    println(Over.run(x => x + 1))
    println(Over.run { x => x match { case 1 => 2 } })
    println(Over.run(_ match { case 1 => 2 }))
    val f: Int => Int = _ + 1
    println(Over.run(f))
    println(Over.shape(s => s match { case Circle(r) => r; case Square(n) => n }))
    println(Over.shape { case Circle(r) => r })
    println(Over.pair(x => x match { case 1 => 2 }, 1))
    println(Over.rest(x => x match { case 1 => 2 }))
    println(Over.only(s => s match { case Circle(r) => r }))
    println(Over.only(s => s match { case Circle(r) => "c" }, "t "))
    println(Built(s => s match { case Circle(r) => r; case Square(n) => n }).tag)
    println(Built({ case Circle(r) => r }, summon[DummyImplicit]).tag)
    val chosen: String = Over.choose(x => x match { case 1 => 2 })
    println(chosen)
    val counted: Int = Over.choose(x => x match { case 1 => 2 })
    println(counted)
    val tupled: Int = Over.pairs((x, y) => x + y)
    println(tupled)
    val direct: String = Over.pairs((x, y) => x + y)
    println(direct)
