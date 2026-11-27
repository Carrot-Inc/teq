// A concrete val overridden by a val, read in the parent's constructor after the parent's own initialiser: the
// read is the subclass's field, not set yet (0), then the subclass's value.
class Parent {
  val x: Int = 5
  println(x)
}
class Child extends Parent { override val x: Int = 9 }
object Main { def main(args: Array[String]): Unit = { println((new Child).x) } }
