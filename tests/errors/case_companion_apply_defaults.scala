// The `apply` a case class's companion gets takes the constructor's defaults, so a written `apply` of
// other parameters with defaults of its own is a second overloaded variant with defaults (dotty's
// `checkNoDoubleDeclaration`); one of the constructor's parameter types replaces the synthesized one (L, P),
// and an abstract case class or one without defaults has none to clash with (M, N). Master accepted K.
case class K(x: Int = 2)
object K:
  def apply(s: String = "s"): K = new K(s.length)
case class L(x: Int = 2)
object L:
  def apply(x: Int): L = new L(x + 1)
case class M(x: Int)
object M:
  def apply(s: String = "s"): M = new M(s.length)
abstract case class N(x: Int = 2)
object N:
  def apply(s: String = "s"): N = new N(s.length) {}
case class P(x: Int = 2)
object P:
  def apply(x: Int = 5): P = new P(x)
// expect: 7:7: error: two or more overloaded variants of method apply have default arguments
