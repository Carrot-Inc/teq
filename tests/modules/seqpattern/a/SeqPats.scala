package spa

case class Rep(name: String, xs: Int*)

object Words:
  def unapplySeq(s: String): Option[Seq[String]] = Some(List(s.take(1), s.drop(2)))

class Node(val name: String, val children: Int*)
object Node:
  def unapplySeq(n: Node): Option[(String, Seq[Int])] = Some((n.name, n.children))

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
  def rep(r: Rep): Int = r match
    case Rep(n, a, rest*) => n.length + a + rest.length
    case Rep(n) => n.length
  def words(s: String): String = s match
    case Words(a, b) => b + a
    case _ => s
  def node(n: Node): Int = n match
    case Node(name, a, rest*) => name.length + a + rest.sum
    case Node(_) => 0
  def onAny(x: Any): Int = x match
    case List(a: Int, b: Int) => a + b
    case Vector(_*) => 1
    case _ => 0
  def arr(a: Array[Int]): Int = a match
    case Array(k, v) => k + v
    case _ => 0
  inline def startsWithOne(s: Seq[Int]): Int = s match
    case Seq(1, _*) => 1
    case _ => 0
  inline def restOf(s: List[Int]): Int = s match
    case List(_, rest*) => rest.length
    case _ => -1
