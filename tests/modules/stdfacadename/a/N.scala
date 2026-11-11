package na

class Node(val name: String, val children: Int*)
object Node:
  def unapplySeq(n: Node): Option[(String, Seq[Int])] = Some((n.name, n.children))
