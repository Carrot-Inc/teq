class Box[A](val x: A) {
  def map[B](f: A => B): Box[B] = { println("map"); new Box(f(x)) }
  def flatMap[B](f: A => Box[B]): Box[B] = { println("flatMap"); f(x) }
}
object P {
  def main(args: Array[String]): Unit = {
    println("nested")
    val b = for { x <- new Box(1); (given Int, y) = (x, 2) } yield summon[Int] + y
    println(b.x)
    println("top")
    val c = for { x <- new Box(1); given Int = x } yield summon[Int] + 1
    println(c.x)
    println("bound")
    val d = for { x <- new Box(1); z @ (given Int, y) = (x, 3) } yield summon[Int] + y + z._2
    println(d.x)
    println("before a generator")
    val e = for { x <- new Box(1); (given Int, y) = (x, 4); w <- new Box(5) } yield summon[Int] + y + w
    println(e.x)
    println("plain")
    val f = for { x <- new Box(1); (u, y) = (x, 6) } yield u + y
    println(f.x)
  }
}
