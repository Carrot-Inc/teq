// A module's synthetic `new O$()` resolves its prefixes: a wildcard of the parent package that
// brings the file's package is used by an object, a case class's companion, an enum, a top-level
// def's package object, an object nested in a class; not by a plain class or trait alone.
package a.b {
  class X
}
package a.b.c1 {
  import a.b.*
  object Use
}
package a.b.c2 {
  import a.b.*
  class Plain
}
package a.b.c3 {
  import a.b.*
  case class Z(i: Int)
}
package a.b.c4 {
  import a.b.*
  trait T
}
package a.b.c5 {
  import a.b.*
  def z = 1
}
package a.b.c6 {
  import a.b.*
  class W {
    object Inner
  }
}
