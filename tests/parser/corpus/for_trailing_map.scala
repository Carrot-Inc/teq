//> using scala 3.8.4
// The trailing map of a for is left out when its body is the generator's own pattern (Scala 3.8).
class M[A](val v: A):
  def map[B](f: A => B): M[B] = { println("map"); M(f(v)) }
  def flatMap[B](f: A => M[B]): M[B] = { println("flatMap"); f(v) }
  def withFilter(p: A => Boolean): M[A] = { println("withFilter"); this }
  def foreach(f: A => Unit): Unit = { println("foreach"); f(v) }
object M:
  def apply[A](v: A): M[A] = new M(v)
@main def run(): Unit =
  println("1"); val a = for x <- M(1) yield x
  println("2"); val b = for x <- M(1) if x > 0 yield x
  println("3"); val c = for (x, y) <- M((1, 2)) yield (x, y)
  println("4"); val d = for (x, y) <- M((1, 2)) yield (y, x)
  println("5"); val e = for x <- M(1); y <- M(2) yield y
  println("6"); val f = for x <- M(1) yield ()
  println("7"); val g = for x <- M(1); y = x + 1 yield y
  println("8"); val h = for x <- M(1) yield (x)
  println("9"); val i = for x <- M(1); y <- M(2) yield (x, y)
  println("10"); val j = for x <- M(1); y <- M(2) if y > 0 yield y
  println("11"); val k = for x <- M((1, 2)) yield x
  println("12"); val l = for (x, (y, z)) <- M((1, (2, 3))) yield (x, (y, z))
  println("13"); val n = for x <- M(1) do println(x)
  println(a.v.toString + b.v + c.v + d.v + e.v + f.v + g.v + h.v + i.v + j.v + k.v + l.v)
