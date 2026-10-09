// The TASTy inspector's own shape over its jars: scala-library's `Quotes` and
// scala3-tasty-inspector's `Tasty`, the function's parameter naming its contextual one.
package tinsp

import java.nio.file.Path
import scala.quoted.Quotes
import scala.tasty.inspector.Tasty

object Inspector:
  def inspect[T](files: List[Path])(fn: (quotes: Quotes) ?=> List[Tasty[quotes.type]] => T): T = ???
