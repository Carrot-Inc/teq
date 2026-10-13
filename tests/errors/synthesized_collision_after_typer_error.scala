// expect: 9:14: error: type mismatch: found "bad", required Int
// expect: 1 error found
// absent: Conflicting definitions
// The synthesized member's collision is erasure's error (`checkNoDoubleDeclaration`), which a
// typer that reported one hides, as scalac's phases stop after it.
abstract case class C(n: Int):
  override def toString: String

val x: Int = "bad"
