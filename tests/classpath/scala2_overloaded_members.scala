// jars: scala-library scala2-lib
// Both alternatives of an overloaded member are to survive the reading of a Scala 2 pickle.
trait InheritedEntries extends scala2lib.overloaded.Entries

object OverloadedMembers {
  def directText(x: scala2lib.overloaded.Entries): String = x.render("text")
  def directNumber(x: scala2lib.overloaded.Entries): String = x.render(8)
  def inheritedText(x: InheritedEntries): String = x.render("text")
  def inheritedNumber(x: InheritedEntries): String = x.render(8)
}
