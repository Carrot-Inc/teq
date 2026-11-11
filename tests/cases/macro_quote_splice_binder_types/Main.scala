case class Crate(value: Int)
@main def run(): Unit = {
  println(new Factory[Crate].duplicate(Crate(3)))
  println(Producer.of[Crate].duplicate(Crate(4)))
  println(projection[Crate].read(Crate(9)))
}
