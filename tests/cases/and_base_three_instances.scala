// `argForParam`'s `AndOrType` arm (Types.scala 2802-2811) over a base of three instances: a trait whose parents give
// its owner three (`All`'s `Box[Int] & Box[String] & Box[Double]`, dotty's `get: => Int & String & Double`, `put(a:
// Int | String | Double)`), a class's own three of a higher-kinded base (`LSeq`'s `FromIterator[Iterable] &
// FromIterator[Seq] & FromIterator[LSeq]`, `fromIterator`'s `LSeq[B]`) and a value of an intersection of three parts
// over two of them (`C[B] & LSeq[B]`).
// scalac 3.8.4 accepts the program,
// and teq is to accept it too.
trait Box[A]:
  def get: A
  def put(a: A): Int
trait IB extends Box[Int]
trait SB extends Box[String]
trait DB extends Box[Double]
trait All extends IB with SB with DB
def f(x: All): Double = x.get
def g(x: All, y: Double): Int = x.put(y)
object H:
  trait Iterable[IA] extends FromIterator[Iterable]
  trait FromIterator[C[X] <: Iterable[X]]:
    def fromIterator[B](it: List[B]): C[B]
  trait Seq[AA] extends Iterable[AA] with FromIterator[Seq]
  trait LSeq[AA] extends Seq[AA] with FromIterator[LSeq]
  class T[SA, C[X] <: Seq[X]](val c: Seq[SA] & FromIterator[C] & LSeq[SA], val d: LSeq[SA]):
    def r1(l: List[SA]): LSeq[SA] = d.fromIterator(l)
    def r2(l: List[SA]): LSeq[SA] = c.fromIterator(l)
    def r3(l: List[SA]): C[SA] = c.fromIterator(l)
@main def run(): Unit = println("ok")
