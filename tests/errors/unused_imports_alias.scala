// The import of a stable value's member makes a local: a use of it uses the import, its making
// does not.
class C { val a = 1; val b = 2 }
object Use {
  def f(c: C): Int = {
    import c.a
    a
  }
  def g(c: C): Int = {
    import c.b
    0
  }
}

// teq: --werror --wunused imports
// expect: unused_imports_alias.scala:10:14: warning: unused import
