package va

object Sum:
  def all(xs: Int*): Int = xs.sum
  def labelled(label: String, xs: String*): String = label + xs.mkString(",")
class Bag(val items: String*)
