//> using platform js
class F0 extends (() => Double):
  def apply(): Double = 1.5

class F1(val k: Int) extends (Int => Double):
  def apply(v: Int): Double = v.toDouble * k
  override def toString = s"F1($k)"

case class F2(k: Int) extends ((Int, Int) => Int):
  def apply(a: Int, b: Int): Int = a + b + k

object Neg extends (Int => Int):
  val calls = scala.collection.mutable.ArrayBuffer[Int]()
  def apply(x: Int): Int =
    calls += x
    -x

trait Vec extends (Int => Double):
  def size: Int
  def sum: Double = (0 until size).map(this).sum

class Ones(val size: Int) extends Vec:
  def apply(i: Int): Double = 1.0

enum Op extends (Int => Int):
  case Twice, Inc
  def apply(x: Int): Int = this match
    case Twice => x * 2
    case Inc => x + 1

@main def main(): Unit =
  val f0: () => Double = new F0
  println(f0())
  println((new F0)())
  val f1: Int => Double = F1(2)
  println(f1(3))
  println(F1(2))
  println(List(1, 2, 3).map(F1(10)).map(_.toString))
  println(f1.andThen(_ + 1)(1))
  val f2: (Int, Int) => Int = F2(1)
  println(f2(3, 2))
  println(F2(1) == F2(1))
  println(Set(F2(1), F2(1), F2(2)).size + " " + F2(1).k)
  println(List(1, 2).map(Neg))
  println(Neg(5) + " " + Neg.calls)
  val v: Vec = Ones(3)
  println(v(0) + v.size + v.sum)
  val anon = new (Int => Int) { def apply(x: Int): Int = x * 3 }
  println(anon(4))
  println(Op.values.toList.map(op => op(5)))
  val ops: List[Int => Int] = List(Op.Twice, Op.Inc)
  println(ops.map(_(1)))
  val g: Int => Int = if f0() > 1 then Op.Twice else Neg
  println(g(21))
  val k = f1 match
    case f: F1 => f.k
    case _ => 0
  println(s"${v.isInstanceOf[Int => Double]} $k")
