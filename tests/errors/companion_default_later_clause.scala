// expect: 7:7: error: two or more overloaded variants of method apply have default arguments
// expect: 1 error found
// The synthesized `apply` takes every constructor clause, so a default in the second one meets a
// written `apply` that does not retract it (`Checking`'s duplicate defaults over all clauses).
case class C(x: Int)(val y: Int = 1)
object C:
  def apply(s: String = ""): C = new C(0)()
