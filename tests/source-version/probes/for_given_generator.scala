class Box[A](val a: A) {
  def map[B](f: A => B): Box[B] = new Box(f(a))
}
def f = for given Int <- new Box(1) yield summon[Int]
