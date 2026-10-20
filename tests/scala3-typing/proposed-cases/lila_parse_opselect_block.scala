// A block argument after a dotted symbolic method name (lila's GameRepo:
// `nonEmptyMod(...) ++ bdoc("$unset" -> finishUnsets.++ { ... })`).
case class Doc(elems: List[String]):
  def ++(doc: Doc): Doc = Doc(elems ++ doc.elems)
  def ++(more: String*): Doc = Doc(elems ++ more)

def bdoc(p: (String, Doc)): Doc = Doc(p._1 :: p._2.elems)

@main def run() =
  val finishUnsets = Doc(List("a"))
  println(Doc(List("s")) ++ bdoc("$unset" -> finishUnsets.++ {
    Doc(List("b"))
  }))
