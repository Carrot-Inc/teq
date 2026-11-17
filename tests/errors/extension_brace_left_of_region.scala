// expect: 7:1: error: expected extension methods
// A brace on the line after an extension's header holds its methods where the line stands in the region the
// header stands in: one left of that region ends the region, and the extension has no methods. scalac:
// "Extension without extension methods" at the brace's line.
object A:
  extension (x: Int)
{
  def twice = x * 2
}
