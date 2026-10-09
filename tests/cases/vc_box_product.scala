// A case class holding a value class gives its box as a Product: productElement, productIterator and the
// generated toString, hashCode and equals take the field's class.
class V(val x: Int) extends AnyVal
case class CV(x: Int) extends AnyVal
class S(val s: String) extends AnyVal
case class H(v: V, c: CV, n: Int)
case class K(s: S, l: List[V])
object Main {
  def main(args: Array[String]): Unit = {
    val h = H(new V(5), CV(6), 7)
    println(h.productIterator.map(_.getClass.getName).mkString(","))
    println(h.productElement(0).getClass.getName); println(h.productElement(1)); println(h)
    println(h == H(new V(5), CV(6), 7)); println(h == H(new V(5), CV(7), 7)); println(h.hashCode == H(new V(5), CV(6), 7).hashCode)
    println(h.copy(v = new V(1)).v.x); println((h: Product).productArity); println(h.productElementName(1))
    val k = K(new S("a"), List(new V(1))); println(k); println(k.productElement(0).getClass.getName); println(k == K(new S("a"), List(new V(1))))
    println(CV(7).productElement(0)); println(CV(7).productIterator.toList); println(CV(7).productArity); println(CV(7).productPrefix)
    println((CV(7): Product).productElement(0)); println(CV(7).productElementName(0)); println(CV(7).canEqual(CV(1)))
    println(CV(7).hashCode == CV(7).hashCode); println(CV(7) == CV(7)); println(CV(7).copy(x = 8))
  }
}
