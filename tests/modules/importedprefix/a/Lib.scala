package importedprefix

// A construction inside the enclosing class through another instance's imported class takes
// that instance (`Typer.newExpr`), and the products state it as the `new`'s prefix.
class O(val n: Int):
  class I(val x: Int):
    def get = n + x
  class J(val x: Int)(val y: Int):
    def get = n + x + y
  def make(other: O): Int =
    import other.{I => R}
    new R(3).get
  def curried(other: O): Int =
    import other.{J => Renamed}
    new Renamed(4)(5).get
