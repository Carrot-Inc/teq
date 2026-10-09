// A lambda whose body is a block of statements ending in a match is no pattern-matching
// anonymous function: scalac warns at the match (E211), which hides that match's own checks;
// a lambda of a tuple's elements binds them in a block around its body, so a match that is its
// body is such a match, and one in braces is neither. A match whose value is one of the block's
// own definitions is ascribed a type without it, and the block ends in no match: no E211, the
// match checked as any.
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape

object Main:
  def main(args: Array[String]): Unit =
    val withVal: PartialFunction[Shape, Int] = s =>
      val k = 1
      s match
        case Circle(r) => r + k
        case Circle(q) => q
    println(withVal.isDefinedAt(Square(1)))
    val withImport: PartialFunction[Shape, Int] = s => { import scala.math.max; s match { case Circle(r) => max(r, 0) } }
    println(withImport.isDefinedAt(Square(1)))
    val before: PartialFunction[Shape, Int] = s =>
      val first = s match
        case Square(n) => n
      s match
        case Circle(r) => r + first
    println(before.isDefinedAt(Square(1)))
    val pairs: List[(Int, Shape)] = List((1, Circle(1)), (2, Circle(2)))
    println(pairs.collect((k, v) => v match { case Circle(r) => r + k }))
    val untupled: PartialFunction[(Int, Shape), Int] = (k, v) => v match { case Circle(r) => r }
    println(untupled.isDefinedAt((1, Square(1))))
    val braced: PartialFunction[(Int, Shape), Int] = (k, v) => { v match { case Circle(r) => r } }
    println(braced.isDefinedAt((1, Square(1))))
    val local: PartialFunction[Shape, Int] = s =>
      val k = 1
      s match
        case Circle(r) => k
    println(local.isDefinedAt(Square(1)))
    println(pairs.collect((k, v) => v match { case Circle(r) => k }))
    println(pairs.collect((k, v) => v match { case Circle(r) => k; case Square(n) => k }))
    println(pairs.collect((k, v) => v match { case Circle(r) => if r > 1 then k else 0 }))
    val ownParam: PartialFunction[Shape, Shape] = s =>
      println("body")
      s match
        case _ => s
    println(ownParam.isDefinedAt(Square(1)))
    val variable: PartialFunction[Shape, Int] = s =>
      var k = 1
      s match
        case Circle(r) => k
    println(variable.isDefinedAt(Square(1)))
    val applied: PartialFunction[Shape, Int] = s =>
      def k(x: Int): Int = x + 1
      s match
        case Circle(r) => k(r)
    println(applied.isDefinedAt(Square(1)))
    val m = Map(1 -> Circle(1))
    println(m.collect(kv =>
      val s = kv._2
      s match
        case Circle(r) => r))
