// An unused importee between kept ones whose last has Scala's trailing comma: scalafix 0.14.7's removal keeps the
// comment after `b` with `a,` and the trailing comma, which parses (the comma a line before the closing brace), so
// teq writes scalafix's edit.
package edits.trailing_comma_kept
object Names { val a = 1; val b = 2; val c = 3 }
import Names.{
  a, // retained comment
  c,
}
object Use { val n = a + c }
