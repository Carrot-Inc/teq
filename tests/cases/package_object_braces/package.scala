package apimodel

package object route {
  val base: String = "/v1"
  type Path = String
  def ep(p: Path): String = base + p
  opaque type Id = Int
  object Id:
    def apply(n: Int): Id = n
    extension (i: Id) def value: Int = i
  extension (s: String) def slash: String = "/" + s
  case class Rank(n: Int)
  given Ordering[Rank] = Ordering.by(_.n)
}
