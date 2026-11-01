// teq: --werror
// An enum case's `override val` parameter overrides its enum's val parameter as the accessor that parameter is
// (RefChecks (1.12), `ParamForwarding.inheritedAccessor` from the case's superclass, its enum): no diagnostic, for a
// case of one parameter list or more, more parameters than the enum's, a singleton case beside, type parameters, a
// using list, a trait mixed in. A case passing its enum another value is tests/source-version's `override_enum_case`
// and tests/cases/override_param_values.
enum F(val s: Int):
  case G(override val s: Int) extends F(s)

enum H(val s: Int):
  case K extends H(1)
  case J(t: String, override val s: Int) extends H(s)

enum Clauses(val s: Int)(val t: String):
  case C(override val s: Int)(override val t: String) extends Clauses(s)(t)

enum Generic[T](val s: T):
  case G[T](override val s: T) extends Generic[T](s)

enum Using(val s: Int)(using val t: String):
  case U(override val s: Int)(using override val t: String) extends Using(s)(using t)

enum UsingOnly(using val s: Int):
  case U(override val s: Int) extends UsingOnly(using s)

trait X
enum WithTrait(val s: Int):
  case W(override val s: Int) extends WithTrait(s) with X

@main def run(): Unit =
  println(F.G(2).s)
  println(H.J("a", 3).s)
  println(H.K.s)
  val c = Clauses.C(7)("seven")
  println(s"${c.s} ${c.t}")
  println(Generic.G("generic").s)
  println(Using.U(8)(using "eight").t)
  println(UsingOnly.U(9).s)
  println(WithTrait.W(10).s)
