// A numeric unary_+ over a reference, a parameter or a field, whose lowering hands on the reference's node, a getClass
// and a numeric widening of a reference: TASTy writes the Select of the member over the reference
// (TreePickler.scala:476-508), the member found on the reference's widened type, which is the widening's source class
// too (Types.scala:1380-1392). getClass's TypeApply keeps its type argument, as written or as inferred, the receiver's
// type widened (Definitions.scala:309-315, TreePickler.scala:542-547). From session b's check of master efddb38de3
// against the head a3bba6320d, and the review of tr/cut-head-tail-c2 e1f120536d (P1-1).
package fix.unaryparams

class UpBox(val n: Int):
  def plus: Int = +n
  def self: Any = this.getClass
  def wide: Double = n
  def explicitThis: Any = this.getClass[this.type]()
object UpLib:
  def int(i: Int): Int = +i
  def long(l: Long): Long = +l
  def double(d: Double): Double = +d
  def float(f: Float): Float = +f
  def byte(b: Byte): Int = +b
  def short(s: Short): Int = +s
  def char(c: Char): Int = +c
  def klass(i: Int): Any = i.getClass
  def boxed(b: UpBox): Any = b.getClass
  def widened(i: Int): Long = i
  def explicitRef(b: UpBox): Any = b.getClass[b.type]()
  def explicitInt(i: Int): Any = i.getClass[i.type]()
  def explicitField(b: UpBox): Any = b.n.getClass[b.n.type]()
  def explicitGeneric[T](b: T): Any = b.getClass[b.type]()
  def explicitWider(b: UpBox): Any = b.getClass[Any]()
  def inferredGeneric[T](b: T): Any = b.getClass
  def inferredLiteral(i: 1): Any = i.getClass
