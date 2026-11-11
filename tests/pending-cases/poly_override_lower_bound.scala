// `RefChecks.checkOverride` of a polymorphic method: a type parameter's lower bound that the overriding member widens
// is an E164 "error overriding method n in trait Box of type [B >: Int](b: B): B" (scalac 3.8.4; a narrower upper
// bound it accepts, `def m[B <: String]` for `[B <: Int]`). Through two instances the inherited lower bound is the
// range's upper end (`[B >: Int & String]`, `mapOverLambda` at -variance), which `[B >: Int | String]` widens: E164
// too. teq accepts both (master too). The closure adversary of the pass on 97f0487ac (round 24).
trait Box[A] { def n[B >: A](b: B): B }
trait IB extends Box[Int] { def n[B >: String](b: B): B = b }
trait IB2 extends Box[Int]
trait SB2 extends Box[String]
trait Both extends IB2 with SB2 { def n[B >: Int | String](b: B): B = b }
