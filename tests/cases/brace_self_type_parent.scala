// A self type written in braces with nothing after its `=>` (`{ self: E => }`), a trait
// restating its parent's self type: the self type conforms to the parent's, as dotty's
// `RefChecks.checkSelfAgainstParents` checks, and a class mixing in both is accepted.
trait E
trait P { self: E => }
trait Q extends P { self: E => }
class C extends Q with E
@main def run(): Unit = println(new C != null)
