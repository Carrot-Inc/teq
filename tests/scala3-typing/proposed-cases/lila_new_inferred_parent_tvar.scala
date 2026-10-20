// lila: `Paginator(adapter = new: def nbResults ...; def slice ..., ...)`
// where the parent of the anonymous class comes from Paginator.apply's AdapterLike[A].
trait AdapterLike[A]:
  def nbResults: Int
  def slice(offset: Int, length: Int): List[A]

final class Paginator[A](val results: List[A], val nb: Int)
object Paginator:
  def apply[A](adapter: AdapterLike[A], currentPage: Int): Paginator[A] =
    new Paginator(adapter.slice(currentPage * 2, 2), adapter.nbResults)

def pager(page: Int): Paginator[String] =
  Paginator(
    adapter = new:
      def nbResults = 5
      def slice(offset: Int, length: Int): List[String] =
        List("a", "b", "c", "d", "e").slice(offset, offset + length)
    ,
    currentPage = page
  )

@main def run() =
  val p = pager(1)
  println(p.results.mkString(",") + " of " + p.nb)
