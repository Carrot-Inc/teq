// A copy default is checked against bounded prototype wildcards from its method info.
final case class Reader[A <: CharSequence](read: Option[A] => Int)
final case class Writer[A <: CharSequence](write: List[A] => String, label: String)
@main def run(): Unit =
  val reader = Reader[String](value => value.fold(0)(_.length))
  println(reader.copy().read(Some("abcd")))
  val writer = Writer[String](_.mkString("/"), "first")
  println(writer.copy(label = "second").write(List("a", "b")))
  println(writer.copy().label)
