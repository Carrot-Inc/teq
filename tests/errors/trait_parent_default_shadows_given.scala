// An empty list given to a trait's implicit clause takes the parameters' defaults, never the search: where a given of a
// parameter's type is in scope, scalac warns that the default shadows it (E220, DefaultShadowsGiven), as it does for a
// method's and a class's (the value is the default's: tests/cases/trait_parent_empty_implicit_defaults has it with no
// given). By a class parent and an anonymous class, and an implicit clause after a using one. A parameter without a
// default after a shadowing one is E171, and then no warning is given for the one before it.
// expect: 21:18: warning: Argument for implicit parameter a was supplied using a default argument.
// expect: 22:18: warning: Argument for implicit parameter n was supplied using a default argument.
// expect: 23:18: error: missing argument for parameter b of constructor TN in trait TN: (implicit a: Int, b: String): TN
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect: 25:16: warning: Argument for implicit parameter a was supplied using a default argument.
// expect: 26:16: warning: Argument for implicit parameter n was supplied using a default argument.
// expect: 27:16: error: missing argument for parameter b of constructor TN in trait TN: (implicit a: Int, b: String): TN
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
class A(val s: String)
given Int = 7
trait TD(implicit a: Int = 44):
  def value: Int = a
trait TU(using a: A)(implicit n: Int = 46):
  def value: String = a.s + n
trait TN(implicit a: Int = 47, b: String)
class S1 extends TD()()
class S2 extends TU(using A("wa"))()
class S3 extends TN()()
object Q:
  val q1 = new TD()() {}
  val q2 = new TU(using A("wb"))() {}
  val q3 = new TN()() {}
