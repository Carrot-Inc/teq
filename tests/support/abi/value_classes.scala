// Item 7 of the JVM ABI alignment: a value class's methods are its
// companion's `m$extension(u, args)`, public and final, with static forwarders in the class and the box's methods
// calling them, its defaults' getters `m$default$N$extension`; a value case class's companion has scalac's erased
// `apply(J)J` and `unapply(J)J`, `copy$extension`, `copy$default$1$extension` and `_1$extension`, and is no
// `Mirror.Product`. The members scalac synthesizes are the companion's too, the box's calling them:
// `hashCode$extension` and `equals$extension`, and a value case class's `toString$extension` and product members.
// abi: plus plus$extension scaled scaled$default$1 scaled$default$1$extension scaled$extension apply unapply copy copy$extension copy$default$1 copy$default$1$extension _1 _1$extension fromProduct scala/deriving/Mirror$Product hashCode hashCode$extension equals equals$extension toString toString$extension canEqual canEqual$extension productArity productArity$extension productPrefix productPrefix$extension productElement productElement$extension productElementName productElementName$extension
package value_classes
class V(val x: Long) extends AnyVal:
  def plus(y: Long): Long = x + y
  def scaled(k: Int = 2): Long = x * k
case class VC(x: Long) extends AnyVal:
  def plus(y: Long): Long = x + y
case class Name(s: String) extends AnyVal
class D(val d: Double) extends AnyVal
class T(val t: Int) extends AnyVal:
  override def hashCode: Int = t % 10
object Holder:
  class Inner(val i: Int) extends AnyVal:
    def plus(j: Int): Int = i + j
