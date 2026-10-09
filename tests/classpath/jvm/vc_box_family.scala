// jars: scala-library
// The box's hashCode and equals as scalac synthesizes them for a value class: a primitive underlying value's
// own hashCode (Double.hashCode), Objects.hashCode of a reference, primitive comparison (NaN unequal, -0.0
// equal to 0.0); a case value class's from its underlying value, and a case class's over boxed fields.
case class CV(x: Int) extends AnyVal
case class CD(d: Double) extends AnyVal
case class CS(s: String) extends AnyVal
class D(val u: Double) extends AnyVal
class L(val u: Long) extends AnyVal
class B(val u: Boolean) extends AnyVal
class C(val u: Char) extends AnyVal
class S(val u: String) extends AnyVal
class W[A](val u: A) extends AnyVal
case class H(d: D, c: CD, n: Int)
case class K(d: D, s: S)
object Main {
  def box[A](x: A): Any = x
  def main(args: Array[String]): Unit = {
    println(CV(1).hashCode); println(box(CV(1)).hashCode); println(CD(1.5).hashCode); println(box(CD(1.5)).hashCode)
    println(CS("a").hashCode); println(box(CS(null)).hashCode); println(box(CS("a")).toString)
    println(box(new D(1.5)).hashCode); println(box(new L(1L << 40)).hashCode); println(box(new B(true)).hashCode); println(box(new C('a')).hashCode)
    println(box(new S("a")).hashCode); println(box(new S(null)).hashCode); println(box(new W(1.5)).hashCode); println(box(new W("a")).hashCode)
    println(box(new D(1.5)).equals(box(new D(1.5)))); println(box(new D(Double.NaN)).equals(box(new D(Double.NaN))))
    println(box(new D(-0.0)).equals(box(new D(0.0)))); println(box(new S("a")).equals(box(new S("a")))); println(box(new S(null)).equals(box(new S(null))))
    println(box(new W(1)).equals(box(new W(1L)))); println(box(new D(1.0)).equals(1.0)); println(box(CD(Double.NaN)).equals(box(CD(Double.NaN))))
    val h = H(new D(1.5), CD(2.5), 3); println(h); println(h.hashCode); println(h == H(new D(1.5), CD(2.5), 3)); println(H(new D(Double.NaN), CD(0.0), 1) == H(new D(Double.NaN), CD(0.0), 1)); println(H(new D(-0.0), CD(-0.0), 1) == H(new D(0.0), CD(0.0), 1))
    val k = K(new D(1.0), new S("x")); println(k); println(k.hashCode); println(K(new D(1.0), new S(null)).hashCode)
    println(CV(3).productElement(0)); println(CV(3).productPrefix); println(CV(3).canEqual(CV(3))); println(CV(3).productArity); println(CV(3).productElementName(0))
    println(CV(3) == CV(3)); println(box(CV(3)) == box(CV(3))); println(CV(3).copy(x = 4))
  }
}
