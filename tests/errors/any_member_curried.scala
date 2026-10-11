// `Any`'s `equals(x$0: Any): Boolean` takes part in the override check with its complete type (dotty's
// `RefChecks.checkOverride`, `compatTypes`): `equals(x: Any)()` has its parameters across two clauses, the same
// member with an incompatible type (scalac's E164 at the trait), where a check of the parameters alone took it for
// `Any`'s and implemented.
// expect: 6:15: error: error overriding method equals in class Any of type (x$0: Any): Boolean: method equals of type (x: Any)(): Boolean has incompatible type
trait T { def equals(x: Any)(): Boolean }
class C extends T
@main def run() = println((new C).equals("x")())
