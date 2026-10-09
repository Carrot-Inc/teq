package fix.seqpat

// Sequence patterns, one of every kind: a companion's `unapplySeq`, a case class of a repeated
// field, an extractor's sequence, the rest named and anonymous.
case class SpRep(name: String, xs: Int*)

object SpWords:
  def unapplySeq(s: String): Option[Seq[String]] = Some(List(s.take(1), s.drop(2)))

class SpNode(val name: String, val children: Int*)
object SpNode:
  def unapplySeq(n: SpNode): Option[(String, Seq[Int])] = Some((n.name, n.children))

object SeqPats:
  def exact(l: List[String]): String = l match
    case List(a, b) => a + b
    case _ => ""
  def named(l: List[Int]): Int = l match
    case List(h, tail*) => h + tail.length
    case _ => 0
  def anon(s: Seq[Int]): Int = s match
    case Seq(x, _*) => x
    case _ => 0
  def nested(l: List[Seq[Int]]): Int = l match
    case List(Seq(a), Seq(b, c)) => a + b + c
    case _ => 0
  def rep(r: SpRep): Int = r match
    case SpRep(n, a, rest*) => n.length + a + rest.length
    case SpRep(n) => n.length
  def words(s: String): String = s match
    case SpWords(a, b) => b + a
    case _ => s
  def node(n: SpNode): Int = n match
    case SpNode(name, a, rest*) => name.length + a + rest.sum
    case SpNode(_) => 0
  def onAny(x: Any): Int = x match
    case List(a: Int, b: Int) => a + b
    case Vector(_*) => 1
    case _ => 0
  def arr(a: Array[Int]): Int = a match
    case Array(k, v) => k + v
    case _ => 0
