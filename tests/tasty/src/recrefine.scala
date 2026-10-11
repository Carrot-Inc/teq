// A refinement's own alias that a sibling names stays a member reference (`T` of `RecThis`), so
// the refinement is a recursive type, `RECtype` over its chain (dotty's `RecType.apply`,
// `closeOver`); one whose members name no sibling stays a plain refinement. The module case
// recmacro reads the same shapes through a macro on either side's products.
package fix.recrefine

class RrBox:
  type T = String
  def f(): T = "own"
  def g(): String = "plain"

object RrShapes:
  type S = AnyRef { type T = String; def f(): T }
  type R = AnyRef { type T; type U = T }
  type P = AnyRef { type T = String; def g(): String }
  def make: S = RrBox()
  def plain: P = RrBox()
