// Assignment-like operators (`>>=`, `+=`) have the lowest precedence and group from the left;
// an operator starting with `=` is an ordinary one, and the receiver of an assignment operator
// may be a method call whose type provides the operator.

final case class Box[A](a: A):
  def >>=[B](f: A => Box[B]): Box[B] = f(a)
  def map[B](f: A => B): Box[B] = Box(f(a))

final case class Cell[A](a: A):
  def map[B](f: A => B): Cell[B] = Cell(f(a))

extension [A](c: Cell[A]) def >>=[B](f: A => Cell[B]): Cell[B] = f(c.a)

extension (a: Int) def ===(b: Int): Boolean = a == b
extension [A](a: Option[A]) def =!=(b: Option[A]): Boolean = a != b

def inc(x: Int): Box[Int] = Box(x + 1)
def dbl(y: Int): Box[Int] = Box(y * 2)

@main def main(): Unit =
  println(Box(1) >>= inc >>= dbl)
  println(Box(1) >>= inc >>= dbl >>= inc)
  println(Box(1).map(_ + 1) >>= inc)
  println(Cell(1).map(_ + 1) >>= (x => Cell(x * 2)))
  println((Cell(1).map(_ + 1)) >>= (x => Cell(x * 2)))
  val cell = Cell(1).map(_ + 1)
  println(cell >>= (x => Cell(x * 2)))
  var c = 1
  c +=
    5
  println(c)
  var t = 0
  t += 1 + 2 * 3
  println(t)
  var n = 2
  n *= 3 max 4
  println(n)
  var xs = List(1)
  xs ::= 2
  println(xs)
  var acc = 0
  List(1, 2, 3).foreach(acc += _ * 2)
  println(acc)
  val ys = List(1, 2, 3)
  println(ys.indexOf(2) === 1)
  println(ys.indexOf(3) === 1)
  val opt = Option(1)
  println(opt.map(_ + 1) =!= Some(2))
  println(opt.map(_ + 1) =!= Some(3))
  val arr = Array(1, 2, 3)
  arr(1) += 10
  println(arr.toList)
