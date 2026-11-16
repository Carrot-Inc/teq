// A constructor is an alternative of a creation where it is accessible at the call (dotty's
// `isAccessibleFrom`, asked of each alternative before the resolution): a hidden one is not chosen, however
// specific; a protected one is reached by a super call: a parent call and an anonymous subclass's creation;
// a qualified one from inside what it names. A trait that extends a class calls no constructor of it, and a
// mirror of a case class is given to a subclass's body. A companion that extends its class reaches the private
// constructor in its own parent call; a class inside the companion calls it through a trait. A parent call whose
// type arguments are written is a super call; one that leaves them to inference is typed ahead of it, where a
// protected constructor is no alternative (the primary takes the two arguments as a tuple).
import scala.deriving.Mirror
package up:
  class G[T] protected (val x: T)
  class H[T](val x: T):
    protected def this(a: T, b: T) = this(a)
  class Hidden(val x: Any):
    protected def this(x: String) = this("secondary".asInstanceOf[Any])
  class HiddenPrivate(val x: Any):
    private def this(x: String) = this("secondary".asInstanceOf[Any])
  class HiddenQualified(val x: Any):
    private[up] def this(x: String) = this("secondary".asInstanceOf[Any])
  class Prot protected (val x: Int)
  class ProtIn protected[up] (val x: Int)
  class Sec(val x: Int):
    protected def this() = this(7)
    private[up] def this(s: String) = this(s.length)
  object InPkg:
    def sec(): Sec = new Sec("four")
    def prot(): ProtIn = new ProtIn(5)
  package sub:
    object InSub:
      def sec(): Sec = new Sec("sixsix")
class Priv private (val x: Int)
trait T extends Priv
object Priv:
  class D extends Priv(8) with T
  def make: Priv = new D
case class M private (x: Int):
  def this() = this(1)
case object M
class MD extends M():
  def make: M = summon[Mirror.ProductOf[M]].fromProduct(Tuple1(9))
class Sub extends up.Sec():
  def this(n: Int) = this()
object Obj extends up.Prot(10)
class Own private (val x: Int)
object Own extends Own(11)
class Thru private (val x: Int = 12)
trait ThruT extends Thru
object Thru:
  class D extends ThruT
  def make: Thru = new D
class GD extends up.G[String]("written")
class HE extends up.H("a", "b")
@main def main(): Unit =
  println(new up.Hidden("argument").x)
  println(new up.HiddenPrivate("argument").x)
  println(new up.HiddenQualified("argument").x)
  println((new up.Prot(1) {}).x)
  println((new up.ProtIn(2) {}).x)
  println((new up.Sec() {}).x)
  println(up.InPkg.sec().x)
  println(up.InPkg.prot().x)
  println(up.sub.InSub.sec().x)
  println(new Sub(0).x)
  println(Obj.x)
  println(Priv.make.x)
  println(new MD().make)
  println(Own.x)
  println(Thru.make.x)
  println(new GD().x)
  println((new up.G[String]("anonymous") {}).x)
  println(new HE().x)
  println((new up.H("c", "d") {}).x)
  println((new up.H[String]("e", "f") {}).x)
