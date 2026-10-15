// A `derives` clause on the line after the class's header, then the body, as indigo's classes write it
// (dotty's `newLinesOptWhenFollowedBy(nme.derives)`): the clause and the body are the class's.
final case class Finder(previous: List[Int], current: Int, next: List[Int])
    derives CanEqual:
  def count: Int = previous.length + 1 + next.length
  def forward: Finder = if next.isEmpty then this else Finder(previous :+ current, next.head, next.tail)
final case class Atlas(names: List[String])
    derives CanEqual {
  def +(other: Atlas): Atlas = Atlas(names ++ other.names)
}
@main def run(): Unit =
  println(Finder(Nil, 1, List(2, 3)).forward.count)
  println(Atlas(List("a")) + Atlas(List("b")))
