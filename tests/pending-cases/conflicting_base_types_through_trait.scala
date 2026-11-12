// A class whose bases conflict through a trait (`Both`'s
// `Box[Int]` and `Box[String]`, which a trait may
// have): scalac 3.8.4 rejects `class Impl extends Both`
// ("cannot be instantiated since it has conflicting base types",
// `Checking.checkInstantiable` over `CheckRealizable`'s `HasProblemBase`),
// as it rejects the anonymous class of `new Both { .. }`.
trait Box[A]:
  def get: A
trait IB extends Box[Int]
trait SB extends Box[String]
trait Both extends IB with SB
class Impl extends Both:
  def get: Int & String = ???
