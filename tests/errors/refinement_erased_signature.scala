// expect: 13:31: error: type mismatch: found C[String], required AnyRef{def f(x: String): String}
// expect: 14:37: error: type mismatch: found C[String], required AnyRef{def f(x: String): String}
// absent: 15:
// A member the refined type's class does not declare is called by reflection, which needs its
// erased parameters to be the refinement's: `C[A].f(x: A)` erases its parameter to `Object`
// (`TypeComparer.hasMatchingMember`'s `sigsOK`, `Signature.consistentParams` of the member's own
// declaration), which no call need reach for the conformance to fail. A parent that declares
// the member, or `Selectable.WithoutPreciseParameterTypes`, lifts it
// (tests/cases/refinement_signature_exemptions.scala).
class C[A] { def f(x: A): String = "ok" }
type R = AnyRef { def f(x: String): String }
def use(c: R): Unit = ()
@main def run(): Unit = { use(new C[String]) }
def assigned(): Unit = { val r: R = new C[String] }
def fine(): Unit = { val r: AnyRef { def f(x: String): String } = new AnyRef { def f(x: String): String = x } }
