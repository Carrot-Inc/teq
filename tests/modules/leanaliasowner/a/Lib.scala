package lao

// Top-level aliases scala-library declares outside their package's `package` object, at the top level of a file of
// their own: `EmptyTuple` (Tuple.scala) and `Typeable` (reflect/Typeable.scala), members of `scala.Tuple$package`
// and `scala.reflect.Typeable$package` (`Desugar.packageObjectName`, Desugar.scala:1933-1936). The lean check
// build's pickles are to name those objects as the aliases' prefixes, where scalac's reader resolves them.
object Lib:
  def pair(x: Int *: Int *: EmptyTuple): Int = x.head
  def empty: EmptyTuple = EmptyTuple
  def named[T](using t: scala.reflect.Typeable[T]): String = "typeable"
  def test[T](x: Any)(using t: scala.reflect.Typeable[T]): Boolean = t.unapply(x).isDefined
