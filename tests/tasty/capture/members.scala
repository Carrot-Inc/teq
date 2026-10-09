package capture.members

case class C[A](a: A, b: Int = 1)
enum Color:
  case Red, Green

class Q:
  def copyIt(c: C[String]): C[String] = c.copy(b = 3)
  def vals = Color.values
  def vo = Color.valueOf("Red")
  def hc(x: Any): Int = x.hashCode
  def pi(c: C[Int]) = c.productIterator
  def sync(x: AnyRef): Int = x.synchronized { 1 }
  def tup(t: (Int, String)) = (t.head, t.tail, t.size, t ++ (1.0, 2L), 1 *: t)
  def plus(i: Int) = +i
  def notTrue = !true
  override def equals(o: Any): Boolean = super.equals(o)
  def gc(x: Any) = x.getClass
