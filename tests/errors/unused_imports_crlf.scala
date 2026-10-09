object Lib { val a = 1; val b = 2 }
object Use {
  import Lib.{a,
    b}
  def f = a
}

// teq: --werror --wunused imports
// expect: unused_imports_crlf.scala:4:5: warning: unused import
