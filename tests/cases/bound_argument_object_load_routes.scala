// An argument written first and bound is evaluated before the object named after it is loaded, by every
// route an object's load runs something: its own template, a base's template, and the arguments of its bases'
// constructors, written or default. An object whose load runs nothing may be written where it stands.
def note(s: String): Int = { println("  " + s); 1 }
def effect(tag: String): Int = { println("  effect " + tag); 1 }
def pair(a: Any, b: Any): Unit = println("  pair called")

// a class base whose parameter has a default
class ClassDefaultBase(x: Int = note("ClassDefault loaded"))
object ClassDefault extends ClassDefaultBase
// an argument with an effect in an inner list, the outermost list empty
trait InnerListBase(x: Int)(y: Int = 1)
object InnerList extends InnerListBase(note("InnerList loaded"))()
// a using parameter
class Token
given Token = { note("UsingParam loaded"); new Token }
trait UsingBase(using t: Token)
object UsingParam extends UsingBase
// a base whose own parent call passes an effect
class Top(x: Int)
class Mid extends Top(note("ParentCall loaded"))
object ParentCall extends Mid
// a base class with a statement, a base trait with a val
class StatementBase { note("BaseStatement loaded") }
object BaseStatement extends StatementBase
trait ValBase { val held: Int = note("BaseVal loaded") }
object BaseVal extends ValBase
// a parent trait's parameter default
trait TraitDefaultBase(x: Int = note("TraitDefault loaded"))
object TraitDefault extends TraitDefaultBase
// a base's secondary constructor, called with an effect
class SecondaryBase(x: Int) { def this() = this(note("Secondary loaded")) }
object Secondary extends SecondaryBase
// a base's secondary constructor that runs a statement, called with a literal
class SecondaryBodyBase { def this(x: Int) = { this(); note("SecondaryBody loaded") } }
object SecondaryBody extends SecondaryBodyBase(1)
// a base's type parameter with a context bound
class Mark[T]
given Mark[Int] = { note("ContextBound loaded"); new Mark[Int] }
class ContextBoundBase[T: Mark]
object ContextBound extends ContextBoundBase[Int]
// a case object of a sealed class whose parameter has a default
sealed abstract class SealedBase(val n: Int = note("SealedCase loaded"))
case object SealedCase extends SealedBase
// an object with a statement of its own
object OwnStatement { note("OwnStatement loaded") }
// objects whose load runs nothing: empty, a plain trait's, a lazy val's, a method's
object Empty
trait Plain
object OfPlain extends Plain
object LazyOnly { lazy val late: Int = note("never read") }
object MethodOnly { def twice(k: Int): Int = k * 2 }
// an object nested in one whose load shows
object Outer { note("Outer loaded"); object Inner }

object Test:
  def main(args: Array[String]): Unit =
    println("a class base's default"); pair(b = effect("b"), a = ClassDefault)
    println("an inner argument list"); pair(b = effect("b"), a = InnerList)
    println("a using parameter"); pair(b = effect("b"), a = UsingParam)
    println("a base's own parent call"); pair(b = effect("b"), a = ParentCall)
    println("a base class's statement"); pair(b = effect("b"), a = BaseStatement)
    println("a base trait's val"); pair(b = effect("b"), a = BaseVal)
    println("a parent trait's default"); pair(b = effect("b"), a = TraitDefault)
    println("a base's secondary constructor"); pair(b = effect("b"), a = Secondary)
    println("a secondary constructor's body"); pair(b = effect("b"), a = SecondaryBody)
    println("a context bound"); pair(b = effect("b"), a = ContextBound)
    println("a sealed class's default"); pair(b = effect("b"), a = SealedCase)
    println("a statement of its own"); pair(b = effect("b"), a = OwnStatement)
    println("an empty object"); pair(b = effect("b"), a = Empty)
    println("a plain trait's object"); pair(b = effect("b"), a = OfPlain)
    println("a lazy val's object"); pair(b = effect("b"), a = LazyOnly)
    println("a method's object"); pair(b = effect("b"), a = MethodOnly)
    println("a nested object"); pair(b = effect("b"), a = Outer.Inner)
    println("end")
