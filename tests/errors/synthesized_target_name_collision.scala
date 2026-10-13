// expect: 9:31: error: Conflicting definitions:
// expect: def foo[A]: String in class C at line 9
// absent: for disambiguation
// A declaration `@targetName` renames to a member the rule synthesizes meets it under that one name
// after erasure (`checkNoDoubleDeclaration` pairs the effective names): E120, as scalac refuses it,
// without the hint to rename one of the two.
import scala.annotation.targetName
case class C(n: Int):
  @targetName("toString") def foo[A]: String = ""
