// expect: 17:16: error: Conflicting definitions:
// expect: def color: String in class Game at line 16 and
// expect: final def color: String in class Game at line 17
// expect: 20:16: error: no eligible member missing at Other.this.chess
// expect: 25:12: error: Conflicting definitions:
// expect: def f(y: Int): Int in class Named at line 26 and
// expect: final def f(x: Int): Int in class Named at line 25
// expect: 3 errors found
// A value export's forwarder of a name the class defines is a further alternative of it, which is a
// double definition where the two erase alike, wildcard or named, reported at the export (dotty's
// checkNoDoubleDeclaration over the declarations in order, the forwarders after the class's own); a
// named selector of no member of the qualifier is no eligible member.
class Info(val ply: Int, val color: String)

class Game(val chess: Info):
  def color: String = "own"
  export chess.*

class Other(val chess: Info):
  export chess.missing

class Fs:
  def f(x: Int): Int = x
class Named(b: Fs):
  export b.f
  def f(y: Int): Int = y
