package other
val items: List[Int] = { println("init items"); List(1) }
object Holder:
  println("init holder")
  val held: List[Int] = List(2)
