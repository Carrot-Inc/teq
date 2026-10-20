// Two arguments to a unary method whose parameter is not a tuple type but which a tuple converts
// to (lila's db dsl `pull(item: ElementProducer)` called as `pull(F.marks, UserMark.alt)`).
import scala.language.implicitConversions

case class Elem(k: String, v: Int)
given Conversion[(String, Int), Elem] = p => Elem(p._1, p._2)

case class Doc(elems: List[Elem]):
  def ++(doc: Doc): Doc = Doc(elems ++ doc.elems)
  def ++(more: Elem*): Doc = Doc(elems ++ more)

def set(item: Elem): Doc = Doc(List(item))
def pull(item: Elem): Doc = Doc(List(item))

@main def run() =
  println(set("enabled" -> 1) ++ pull("marks", 2))
