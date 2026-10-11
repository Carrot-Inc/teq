package besideany

// An overload beside `Any`'s `toString`, `hashCode` or `equals` is another alternative (dotty's
// `matchingMember` keeps both): read from the products it keeps its own name, and the rule's
// `toString` of a case class, or a class's own, still answers `println`.
case class D(x: Int):
  def toString(n: Int): String = "overload " + n
  def hashCode(salt: String): Int = salt.length
  def equals(a: Int, b: Int): Boolean = a == b
class P:
  override def toString: String = "P"
  def toString(prefix: String): String = prefix + "P"
