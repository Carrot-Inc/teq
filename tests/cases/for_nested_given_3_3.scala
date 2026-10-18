// `Desugar.makeFor` packs aliases with the element by a `map` of their own where one alias's pattern
// `isNestedGivenPattern` (`hasGivenBind` below a `Bind`'s body: `(given Int, y) = ..`), even from 3.8, where better fors
// put the other aliases into the generator's lambda; a top-level `given Int = x` nests none. Before 3.8 every run of
// aliases is packed.
// teq: --source 3.3
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
