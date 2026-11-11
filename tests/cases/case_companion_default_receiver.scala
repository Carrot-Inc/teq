case class Sample(value: Int = Sample.seed)(val label: String = Sample.label(value))
object Sample:
  val seed: Int = 37
  def label(value: Int): String = s"value=$value"
  def inside: Sample = Sample()()

object Container:
  case class Entry(value: Int = Entry.seed)
  object Entry:
    val seed: Int = 41
    def inside: Entry = Entry()

@main def run(): Unit =
  val a = Sample()()
  println(s"$a ${a.label}")
  val b = new Sample()()
  println(s"$b ${b.label}")
  println(Sample.inside)
  println(Container.Entry())
  println(Container.Entry.inside)
