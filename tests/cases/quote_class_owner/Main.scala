class Receiver(val offset: Int):
  def cell: IntCell = CellFactory.make(offset + 4)

@main def run(): Unit =
  val first = new Receiver(10).cell
  val second = new Receiver(23).cell
  println(first.read)
  println(second.read)
