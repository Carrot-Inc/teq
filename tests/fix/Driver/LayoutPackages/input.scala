/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// The tree of package clauses and imports as scalameta 4.17.0 lays it out (scalameta.txt): every clause's statements in a
// `Pkg.Body`, from its braces or its colon or else its first statement, a block inside the clause around it, an
// importee's name a `Name.Indeterminate`.
package layout
import scala.collection.mutable.{ListBuffer, Map as MMap, Set as _, *} // assert: DumpLayout.layout
import scala.util.Try
package inner {
  import scala.util.Success
  package deeper {
    import scala.util.Failure
  }
}
package other:
  import scala.util.Either
