// The oracle's explicit-nulls fixture (README.md): compiled by scalac under `-Yexplicit-nulls`, so that an inferred
// type holds a `FlexibleType` (a Java method's result, `(String)?`), the one kind only that mode makes.
package repr.rec

object Nulls:
  val flexible = java.lang.String.valueOf(1)
