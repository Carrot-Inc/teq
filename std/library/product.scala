// What a case class, a case object, an enum case or a tuple is under `--std=scala-library`:
// the same trait the lean std declares, whose members the runtime answers for the classes the
// compiler builds, in place of scala-library's abstract declarations.
package scala

trait Product:
  @js("$productArity($0)")
  @jvm("$0:L invokeinterface scala/Product.productArity()I")
  def productArity: Int
  @js("$productElement($0, $1)")
  @jvm("$0:L $1:I invokeinterface scala/Product.productElement(I)Ljava/lang/Object;")
  def productElement(n: Int): Any
  @js("$productPrefix($0)")
  @jvm("$0:L invokeinterface scala/Product.productPrefix()Ljava/lang/String;")
  def productPrefix: String
  def productIterator: Iterator[Any] = productIteratorImpl(this)
  @js("$productElementName($0, $1)")
  @jvm("$0:L $1:I invokeinterface scala/Product.productElementName(I)Ljava/lang/String;")
  def productElementName(n: Int): String
  def productElementNames: Iterator[String] = productElementNamesImpl(this)
  def canEqual(that: Any): Boolean = canEqualImpl(this, that)

def productIteratorImpl(p: Product): Iterator[Any] = Iterator.tabulate(p.productArity)(p.productElement)
def productElementNamesImpl(p: Product): Iterator[String] = Iterator.tabulate(p.productArity)(p.productElementName)
def canEqualImpl(p: Product, that: Any): Boolean = that != null && p.getClass == that.getClass
