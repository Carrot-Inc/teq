// expect: 32:5: error: value tag is not a member of Int
// expect: 35:5: error: value tag is not a member of String
// expect: 40:5: error: value wow is not a member of String
// expect: 46:7: error: value tag is not a member of String
// expect: 52:7: error: value wow is not a member of Int
// The nearest extension method of a name hides the enclosing scopes' whatever its receiver (dotty's
// `findRef` for `tryExtension`): an inner import's `tag` on String hides an outer import's on Int; a
// definition of the file's package in the file beats the file's import; a named import beats the
// wildcard import of its scope; and an import cannot shadow a definition of an enclosing scope,
// whose ambiguity fails the lexical attempt, as does a wildcard import against a named import of
// an enclosing scope. Each selection is then no member of its receiver (scalac's E008, with the
// extension tried).
object Outer:
  extension (n: Int) def tag: String = "outer"
object Inner:
  extension (s: String) def tag: String = "inner"
object One:
  extension (s: String) def wow: String = "one"
object Two:
  extension (n: Int) def wow: String = "two"

extension (b: Boolean) def tag: String = "top"
object X:
  extension (s: String) def tag: String = "x"

import X.*

object Shadow:
  import Outer.*
  def test: String =
    import Inner.*
    1.tag
object TopLevel:
  def test: String =
    "s".tag
object Named:
  import One.*
  import Two.wow
  def test: String =
    "five".wow
object Enclosing:
  extension (n: Int) def tag: String = "enclosing"
  object P:
    def test: String =
      import Inner.*
      "s".tag
object OuterNamed:
  import Two.wow
  object Inner:
    def test: String =
      import Outer2.*
      5.wow
object Outer2:
  extension (n: Int) def wow: String = "outer2"
