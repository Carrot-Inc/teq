//> using scala 3.8.4
class Table[K, V](val rows: List[(K, V)]):
  def map[K2, V2](f: ((K, V)) => (K2, V2)): Table[K2, V2] = Table(rows.map(f))
  def map[B](f: ((K, V)) => B): List[B] = rows.map(f)
  def on(f: String => Unit): String = "string handler"
  def on(f: (String, Int) => Unit): String = "pair handler"
  def pick(f: Int => Boolean): String = "predicate"
  def pick(f: PartialFunction[Int, String]): String = "partial"
  def fold[B](z: B)(f: (B, V) => B): B = rows.foldLeft(z)((b, kv) => f(b, kv._2))
  def fold(f: (V, V) => V): V = rows.map(_._2).reduce(f)
  override def toString = rows.mkString("Table(", ", ", ")")

@main def run(): Unit =
  val t = Table(List("a" -> 1, "b" -> 2))
  println(t.map(kv => (kv._2, kv._1)))
  println(t.map(kv => kv._1 + kv._2))
  println(t.map((k, v) => (k, v * 2)))
  println(t.map((k, v) => k * v))
  println(t.on(s => println(s)))
  println(t.on((s, n) => println(s * n)))
  println(t.pick(_ > 1))
  println(t.pick { case 1 => "one" })
  println(t.fold(0)(_ + _))
  println(t.fold(_ + _))
  println(t.fold("")((acc, v) => acc + v))
