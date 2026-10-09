package capture.lowerings

trait Sam { def run(x: Int): Int }
trait Show[A] { def show(a: A): String }
class Holder { var v: Int = 0 }
given Show[Int] with
  def show(a: Int): String = a.toString

class P:
  final val K = 7
  def k = 1 + 2
  def kk = K * 2
  def widen(i: Int): Long = { val l: Long = i; l + i }
  def mixed(i: Int, c: Char): Int = i + c
  def assign(h: Holder): Unit = { h.v = 3; h.v += 1 }
  def sam: Sam = x => x + 1
  def anon: Sam = new Sam { def run(x: Int) = x * 2 }
  def useGiven: String = summon[Show[Int]].show(1)
  def eqs(a: String, b: String, x: Int, y: Long): Boolean = a == b && (a eq b) && x == y && a != null
  def neg(i: Int, b: Boolean): Int = if !b then -i else ~i
  def conv(i: Int): Double = i.toDouble + i.toLong
  def arr: Array[Int] = Array(1, 2, 3)
  def tup = (1, "a")
  def partial: PartialFunction[Int, Int] = { case 1 => 2 }
  def asI(x: Any): String = x.asInstanceOf[String]
  def asI2(x: Long): Int = x.asInstanceOf[Int]
  def ascr(x: Int): Any = (x: Any)
  def seqp(xs: Seq[Int]): Int = xs match { case Seq(a, rest*) => a; case _ => 0 }
  def listp(xs: List[Int]): Int = xs match { case a :: _ => a; case Nil => 0 }
  def patval: Int = { val (a, b) = (1, 2); a + b }
  def lzy: Int = { lazy val z = 3; z }
  def tailr(n: Int): Int = { @annotation.tailrec def go(i: Int, acc: Int): Int = if i == 0 then acc else go(i - 1, acc + i); go(n, 0) }
  def charAdd(c: Char): String = c + "x"
  def boolAdd(b: Boolean): String = "" + b
  def interpRaw(x: Int) = raw"a\n$x"
  def byNameClosure(f: => Int): () => Int = () => f
  def optionMap(o: Option[Int]) = o.map(_ + 1).getOrElse(0)
  def ctxFn: Int ?=> Int = summon[Int] + 1
