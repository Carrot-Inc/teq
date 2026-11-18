// A library package's top-level members, members of its file's package object on the class path, use the
// package's imports: an extension through `p.*`, a given through `p.given` and through its name; a selector that
// renames the given is not used by a search, as scalac's CheckUnused matches it by the name the tree uses.
object UseExtension {
  import toplevellib.*
  def loud = "hi".shout
}
object UseGiven {
  import toplevellib.given
  def answer = summon[Int]
}
object UseNamedGiven {
  import toplevellib.number
  def seven = summon[Long]
}
object Unused {
  import toplevellib.given
  def plain = 1
}
object RenamedGiven {
  import toplevellib.{number as n}
  def seven = summon[Long]
}

// jars: scala-library toplevel-lib
// teq: --werror --wunused imports --target jvm
// expect: unused_imports_classpath_toplevel_members.scala:17:22: warning: unused import
// expect: unused_imports_classpath_toplevel_members.scala:21:23: warning: unused import
// absent: unused_imports_classpath_toplevel_members.scala:5:
// absent: unused_imports_classpath_toplevel_members.scala:9:
// absent: unused_imports_classpath_toplevel_members.scala:13:
