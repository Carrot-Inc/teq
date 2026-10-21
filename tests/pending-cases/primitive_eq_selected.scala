// `==` and `!=` selected and applied (on an Int here, on any receiver alike), as DisableSyntax's noUniversalEquality fixture writes them: scalac
// accepts them, teq reports "value == is not a member of Int" (tests/fix/REFERENCES.md).
@main def run(): Unit =
  println(1.==(2))
  println(1.!=(2))
