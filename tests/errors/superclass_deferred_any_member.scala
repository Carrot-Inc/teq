// A superclass's own deferred `toString` hides `Any`'s among a subclass's members (dotty's
// `computeMembersNamed`), so nothing implements it (`RefChecks.isImplemented`), as scalac reports;
// a trait's meets the concrete one of the superclass chain, the concrete preferred
// (`Denotations.mergeSingleDenot`), which implements it.
abstract class A { def toString: String }
class B extends A
// expect: 6:7: error: class B needs to be abstract, since def toString: String in class A is not defined
trait T { def toString: String }
class C extends T
abstract class P extends T
class Q extends P
// absent: class C needs to be abstract
// absent: class Q needs to be abstract
