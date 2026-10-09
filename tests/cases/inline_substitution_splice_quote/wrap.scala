// A splice whose code is a quote that calls a macro: the call is kept in the quote and expanded
// where the splice's expansion stands, as scalac keeps a quote's inline calls for the place it is
// spliced; the retype path, which types the code again at the call, expands it there and finds no
// `Quotes` for the macro. scalac prints the lines of the .expected file.
import scala.quoted.*
object Wrap:
  inline def direct: String = ${ '{ Pos.here } }
  inline def direct2(inline s: String): String = ${ '{ s + Pos.here } }
  inline def outer: String = direct
