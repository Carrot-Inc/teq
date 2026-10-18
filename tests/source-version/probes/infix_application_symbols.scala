class C {
  val e: Int =:= Int = summon[Int =:= Int]
  val u: String ?=> Int => Int = x => x + 1
  override def equals(that: Any): Boolean = true
}
object P {
  given String = ""
  def main(args: Array[String]): Unit = {
    val c = new C
    val o = new Object
    println(c eq c)
    println(c ne o)
    println(c equals o)
    println(o synchronized 1)
    println(c e 1)
    println(c u 1)
    println(Array(1, 2) apply 0)
    println("abc" charAt 0)
    println("a" compareTo "b")
    println("abc" concat "d")
    println((1 to 3).length)
    println(List(1) map (_ + 1))
    println(Option(1) getOrElse 2)
    println(1 compare 2)
  }
}
