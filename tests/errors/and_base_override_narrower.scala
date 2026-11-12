// The reverse of tests/cases/and_base_override_through_instances: `get` seen from `Narrow`, whose owner `Both`'s parents
// give at two instances, is `=> Int & String` (RefChecks' `self.memberInfo(other)`, the range's upper end at the
// result's variance); `def get: Int` overrides it with a wider type,
// and scalac 3.8.4 rejects it (E164, "error
// overriding method get in trait Box of type => Int & String; method get of type => Int has incompatible type").
// teq is to reject it; the expectation's message shows the declared type.
// expect: 15:7: error: error overriding method get in trait Box of type A: method get of type Int has incompatible type
// expect: 1 error found
trait Box[A]:
  def get: A
trait IB extends Box[Int]
trait SB extends Box[String]
trait Both extends IB with SB
trait Narrow extends Both:
  def get: Int = 1
