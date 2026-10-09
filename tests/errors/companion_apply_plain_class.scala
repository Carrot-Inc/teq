// A plain class's constructor proxy is no member of its companion to name unqualified, where a
// case class's synthesized `apply` is (tests/cases/companion_apply_unqualified.scala).
class C()
object C:
  val c = apply()
// expect: not found: apply
