// expect: 40:29: error: value secret is not a member of Parcel
// expect: 45:20: error: not found: to
// expect: 46:22: error: not found: secret
// expect: 46:31: error: not found: hidden
// expect: 47:29: error: method draft in class Letter cannot be accessed as a member of Letter from class Parcel; protected method draft can only be accessed from class Parcel or one of its subclasses
// expect: 50:23: error: method address in class Letter is accessed from super. It may not be abstract unless it is overridden by a member declared `abstract' and `override'
// expect: 51:23: error: method seal in trait Sealable is accessed from super. It may not be abstract unless it is overridden by a member declared `abstract' and `override'
// expect: 52:20: error: super may be not be used on value weight
// expect: 53:19: error: super may be not be used on lazy value route
// expect: 54:22: error: value nothing is not a member of the parents of class Parcel
// expect: 55:21: error: Sealable2 does not name a parent of class Parcel
// expect: 56:19: error: Letter2 does not name a parent of class Parcel
// expect: 58:62: error: method cost needs `override` modifier to override method cost in class Letter
// expect: 59:65: error: value weight needs `override` modifier to override value weight in class Letter
// expect: 60:72: error: method id cannot override final member method id in class Letter
// expect: 61:71: error: method weight needs to be a stable, immutable value to override value weight in class Letter
// expect: 62:72: error: value route must be declared lazy to override lazy value route in class Letter
// expect: 63:77: error: lazy value weight may not override non-lazy value weight in class Letter
// expect: 64:72: error: error overriding variable notes in class Letter of type String;
// expect: 65:69: error: method absent overrides nothing
// expect: 66:77: error: method cost has weaker access privileges than method cost in class Letter; it should be public
// expect: 68:46: error: method seal cannot override final member method seal in class Sealed2
// expect: 70:25: error: method draft in class Letter cannot be accessed as a member of Letter from object Office; protected method draft can only be accessed from class Letter or one of its subclasses
// expect: 71:28: error: value margin in class Letter cannot be accessed as a member of Parcel from object Office; protected value margin can only be accessed from class Letter or one of its subclasses
// expect: 76:56: error: Super call cannot be emitted: the selected method m is declared in class Base1, which is not the direct superclass of class Skips
// expect: 25 errors found
trait Stamped { def stamp = "stamped" }
trait Sealable { def seal: String }
abstract class Letter(to: String) {
  def cost = 1
  val weight = 2
  lazy val route = "road"
  var notes = ""
  final def id = 7
  def address: String
  protected def draft = "draft"
  protected val margin = 3
  private val secret = 4
  private def hidden = 5
  def peek(other: Parcel) = other.secret
}
class Parcel extends Letter("p") with Stamped with Sealable {
  def address = "somewhere"
  def seal = "wax"
  def readsParam = to
  def readsPrivate = secret + hidden
  def viaOther(o: Letter) = o.draft
  def viaOwn(o: Parcel) = o.draft + draft + this.margin
  def viaSuper = super.draft + super.stamp + super[Stamped].stamp
  def abstractSuper = super.address
  def abstractTrait = super.seal
  def valueSuper = super.weight
  def lazySuper = super.route
  def missingSuper = super.nothing
  def wrongParent = super[Sealable2].seal
  def notParent = super[Letter2].cost
}
class NoOverride extends Letter("n") { def address = ""; def cost = 2 }
class NoOverrideVal extends Letter("n") { def address = ""; val weight = 3 }
class FinalMember extends Letter("n") { def address = ""; override def id = 8 }
class DefOverVal extends Letter("n") { def address = ""; override def weight = 3 }
class ValOverLazy extends Letter("n") { def address = ""; override val route = "air" }
class LazyOverVal extends Letter("n") { def address = ""; override lazy val weight = 3 }
class VarOverride extends Letter("n") { def address = ""; override var notes = "x" }
class Nothing1 extends Letter("n") { def address = ""; override def absent = 1 }
class Weaker extends Letter("n") { def address = ""; override protected def cost = 2 }
class Sealed2 extends Parcel { final override def seal = "glue" }
class Sealed3 extends Sealed2 { override def seal = "tape" }
object Office {
  def open(l: Letter) = l.draft
  def measure(p: Parcel) = p.margin
}
class Base1 { def m = 1 }
class Base2 extends Base1 { override def m = 2 }
trait Side extends Base1
class Skips extends Base2 with Side { override def m = super[Side].m }
