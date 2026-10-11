// `Any`'s `equals(Any)` implements no abstract `equals(s: String)`: dotty's `RefChecks.isImplemented` compares the
// parameter types of the member it finds (`matchesLoosely(.., alwaysCompareTypes = true)`), where an arity alone
// would take it implemented and the JVM throw `AbstractMethodError`; a case class's synthesized `equals` is `Any`'s.
// expect: 7:7: error: class C needs to be abstract, since def equals(s: String): Boolean in trait T is not defined
// expect: 8:12: error: class D needs to be abstract, since def equals(s: String): Boolean in trait T is not defined
trait T { def equals(s: String): Boolean }
class C extends T
case class D(x: Int) extends T
@main def run() = println((new C).equals("x"))
