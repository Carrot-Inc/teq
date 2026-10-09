// A downstream of Inspector.scala: a call, and one whose lambda declares the parameter's type,
// `Tasty` being invariant: the signature exactly.
package tinspuse

import java.nio.file.Path
import scala.quoted.Quotes
import scala.tasty.inspector.Tasty

object UseInspector:
  def call: Int = tinsp.Inspector.inspect(List.empty[Path])(ts => ts.size)
  def exact: Int = tinsp.Inspector.inspect(List.empty[Path])((quotes: Quotes) ?=> (ts: List[Tasty[quotes.type]]) => ts.size)
