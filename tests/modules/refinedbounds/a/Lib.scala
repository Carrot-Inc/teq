package rba

// A type member read through a prefix erases as the member the prefix has by that name where that is an alias, else
// as its declaration, as declared: a refinement looser than the declaration leaves `c.T` the declared `String`, a
// member bounded by an alias of `V | V` erases as the declared union does (`Object`), intersected too, and an
// intersection keeps the side whose bound is the narrower.
class V(val x: Int) extends AnyVal

object Types:
  type U = V | V

trait Bounded:
  type T <: String
trait Free:
  type T
trait AbsU:
  type T <: Types.U

object Api:
  def looser(c: Bounded { type T })(x: c.T): c.T = x
  def wider(c: Bounded { type T <: AnyRef })(x: c.T): c.T = x
  def both(c: Bounded & (Free { type T }))(x: c.T): c.T = x
  def depU(a: AbsU)(x: a.T): a.T = x
  def interU(c: AbsU & Free)(x: c.T): c.T = x
