// The library half of tests/cases/lazy_inline_unapply: an inline `unapply` declaring a local lazy val, expanded in a
// pattern of another file of the build.
package lib

object Twice:
  inline def unapply(x: Int): Option[Int] = { lazy val doubled: Int = x * 2; if x > 0 then Some(doubled) else None }
