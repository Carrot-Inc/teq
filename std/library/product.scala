// What a case class, a case object, an enum case or a tuple is under `--std=scala-library`:
// scala-library's `Product` over its `Equals`, whose members the runtime answers for the classes
// the compiler builds where they define none (dotty's `SyntheticMembers`), in place of the
// library's abstract declarations.
package scala

trait Product extends Any with Equals:
  @js("$productArity($0)")
  @jvm("$0:L invokeinterface scala/Product.productArity()I")
  def productArity: Int
  @js("$productElement($0, $1)")
  @jvm("$0:L $1:I invokeinterface scala/Product.productElement(I)Ljava/lang/Object;")
  def productElement(n: Int): Any
  @js("$productPrefix($0)")
  @jvm("$0:L invokeinterface scala/Product.productPrefix()Ljava/lang/String;")
  def productPrefix: String
  def productIterator: Iterator[Any] = Iterator.tabulate(productArity)(productElement)
  @js("$productElementName($0, $1)")
  @jvm("$0:L $1:I invokeinterface scala/Product.productElementName(I)Ljava/lang/String;")
  def productElementName(n: Int): String
  def productElementNames: Iterator[String] = Iterator.tabulate(productArity)(productElementName)
  @js("$canEqual($0, $1)")
  @jvm("$0:L $1:L invokeinterface scala/Equals.canEqual(Ljava/lang/Object;)Z")
  def canEqual(that: Any): Boolean
