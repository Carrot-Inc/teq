package fakes
object Fakes:
  val p1 = [T] => (t: T) => t
  val p2 = [A, B] => (a: A, b: B) => (a, b)
  val p3 = [C] => (c: C) => List(c)
  val d1 = (x: String) => x.length
  def m(s: String)(using n: Int): Int = n
  val p4 = [D] => (d: D) => Option(d)
  val p5 = [E] => (e: E) => Set(e)
