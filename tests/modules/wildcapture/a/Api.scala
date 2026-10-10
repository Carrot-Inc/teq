package wca

// A lambda over values of a wildcard type: the member a selection reaches through the wildcard's
// capture, which the writer writes as the wildcard again (`Box[?]`, `Box[? <: Named]`) where scalac's
// tree keeps it, and as its bound where a capture would escape.
trait Named:
  def name: String

class Box[A](val n: Int, val item: A)

object Api:
  def sizes(xs: List[Box[?]]): List[Int] = xs.map(_.n)
  def names(xs: List[Box[? <: Named]]): List[String] = xs.map(_.item.name)
  def firsts(xs: List[Box[?]]): List[Any] = xs.map(b => b.item)
