// lila: HeapSort.topN(xs, n)(using ord)   [topN(...)(using ord: Ordering[T], bf: BuildFrom[...])]
//       relayHome.getJson(1)(using Config(html = false))   [getJson(page)(using Config, Translate)]
// An explicit `(using ...)` argument list shorter than the clause: scalac fills the missing
// trailing parameters by implicit search (Applications.tryDefault -> implicitArgTree).
class Translate(val lang: String)
case class Config(html: Boolean)

def getJson(page: Int)(using c: Config, t: Translate): String =
  s"page $page html=${c.html} lang=${t.lang}"

@main def run() =
  given Translate = Translate("en")
  println(getJson(1)(using Config(html = false)))
