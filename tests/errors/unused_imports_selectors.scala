// Plain, renamed and wildcard selectors; a wildcard beside a named selector that takes its name.
object Lib { val used = 1; val unused = 2; class Shadow; def other = 3 }
object Use {
  import Lib.{used, unused, Shadow as Sh}
  import Lib.*
  def f = used
}
object Renamed {
  import Lib.{used => u2, other as o}
  def g = o
}
object Wild {
  import Lib.*
  def h = other
}
object Lines {
  import Lib.{
    used,
    unused
  }
  def k = used
}
object Clauses {
  import Lib.used, Lib.unused
  def m = unused
}

// teq: --werror --wunused imports
// expect: unused_imports_selectors.scala:4:21: warning: unused import
// expect: unused_imports_selectors.scala:4:29: warning: unused import
// expect: unused_imports_selectors.scala:5:14: warning: unused import
// expect: unused_imports_selectors.scala:9:15: warning: unused import
// expect: unused_imports_selectors.scala:19:5: warning: unused import
// expect: unused_imports_selectors.scala:24:14: warning: unused import
