// lila: `paginatorWriteNoNbResults.writes(tours.map(tourWithAnyRound))` where
// `def paginatorWriteNoNbResults[A](using Writes[A]): OWrites[Paginator[A]]`.
trait Writes[A]:
  def writes(a: A): String
given Writes[Int] = i => s"int $i"
given Writes[String] = s => s"str $s"

final class Page[A](val items: List[A])
def pageWrites[A](using w: Writes[A]): Writes[Page[A]] =
  p => p.items.map(w.writes).mkString("[", ",", "]")

@main def run() =
  val p = Page(List("a", "b"))
  println(pageWrites.writes(p))
