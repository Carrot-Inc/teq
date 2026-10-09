package libb
import scala.collection.mutable.{ListBuffer as LB}
import liba.A
trait Parent:
  def base: Int = 1
class Cases extends Parent:
  def helper(x: Int = 1): Int = x
  def local(p: Int): Int =
    val n = p
    def inner(q: Int): Int = n + q
    inner(p)
  def uses: Int =
    val b = new LB[Int]()
    b.addOne(helper(x = 2))
    this.helper() + super.base
  type Lam = [A] =>> List[A]
  def refined(x: Parent { type Out = Int }): x.Out = 1
  inline def twice(inline x: Int): Int = x + x
  def expanded: Int = twice(helper(3))
  def bounded[T: Ordering](x: T, y: T): T = if summon[Ordering[T]].lt(x, y) then y else x
  def chained(xs: List[Int]): Int = xs.map(_ + 1).filter(_ > 2).sum
  def made: Int = A.make.hello
trait Base:
  def value: Int = 0
class Left extends Base:
  override def value: Int = 1
class Right extends Base:
  override def value: Int = 2
object Branches:
  def chosen(flag: Boolean, l: Left, r: Right): Int = (if flag then l else r).value
  def matched(flag: Boolean, l: Left, r: Right): Int = (flag match { case true => l; case false => r }).value
  def caller: Int =
    val n = Cases.helper2(1)
    n
  def localCall: Int =
    def inner2(x: Int): Int = x
    inner2(x = 1)
object Cases:
  def helper2(x: Int): Int = x
class Box[+A](val get: A)
object Receivers:
  def joined(flag: Boolean, l: Box[Left], r: Box[Right]): Int = (if flag then l else r).get.value
  def bounded2[T <: Box[Left]](x: T): Int = x.get.value
  def meet(x: Base & Left): Int = x.value
  def refinedBox(x: Box[Left] { def extra: Int }): Int = x.get.value
  def asInstanceOf[T](tag: Int): Right = new Right
  def custom: Int = asInstanceOf[Int](1).value
trait Wide:
  def get: Base
trait Narrow:
  def get: Left
trait Carrier:
  type Item <: Base
  def get: Item
object Receivers2:
  def unrelated(x: Wide & Narrow): Int = x.get.value
  def unrelatedReverse(x: Narrow & Wide): Int = x.get.value
  def refinedCarrier(x: Carrier { type Item = Left }): Int = x.get.value
  def refinedVal(x: Box[Base] { val get: Left }): Int = x.get.value
object Opaques:
  opaque type Hidden <: Base = Left
  def hidden(x: Hidden): Int = x.value
object OutsideOpaque:
  def opaqueUse(x: Opaques.Hidden): Int = x.value
class ValWide(val get: Base)
trait ValNarrow:
  val get: Left
object Receivers3:
  def boundedVal[T <: Box[Base] { val get: Left }](x: T): Int = x.get.value
  def valConcrete(x: ValWide & ValNarrow): Int = x.get.value
trait ThisNarrow extends Wide:
  self: Narrow =>
  def read: Int = this.get.value
trait Generic[A <: Base]:
  def get: A
trait GLeft extends Generic[Left]
trait GBase extends Generic[Base]
trait GIndirect[A <: Base] extends Generic[A]
trait GA extends GIndirect[Left]
trait GB extends GIndirect[Left]
trait OtherGet:
  def get: Base
trait SubNarrow extends Narrow
trait SubWide extends Wide
trait SubOther extends OtherGet
abstract class Mix1 extends Wide, Narrow
abstract class Mix2 extends Narrow, Wide:
  override def get: Left
object Receivers4:
  type Ref2 = Box[Base] { val get: Left }
  def same1(x: Box[Base] & Ref2): Int = x.get.value
  def same3(x: Box[Base] & Box[Left]): Int = x.get.value
  def generic1(x: GBase & GLeft): Int = x.get.value
  def generic3(x: GIndirect[Base] & GIndirect[Left]): Int = x.get.value
  def genericJoin(flag: Boolean, a: GA, b: GB): Int = (if flag then a else b).get.value
  def typeMeet(x: Carrier { type Item = Base } & Carrier { type Item = Left }): Int = x.get.value
  def n5(x: (Wide & OtherGet) & (SubNarrow & SubWide)): Int = x.get.value
  def n6(x: (Narrow & Wide) & (SubOther & SubNarrow)): Int = x.get.value
  def mix(flag: Boolean, a: Mix1, b: Mix2): Int = (if flag then a else b).get.value
object Receivers5:
  def fixed[A <: Base, B <: Left](x: Carrier { type Item = A } & Carrier { type Item = B }): Int = x.get.value
  def fixedReverse[A <: Base, B <: Left](x: Carrier { type Item = B } & Carrier { type Item = A }): Int = x.get.value
