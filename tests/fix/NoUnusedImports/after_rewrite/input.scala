/*
rules = [EmptyCollectionsUnified, NoUnusedImports]
*/
// The design pass's case: scalac 3.8.4 finds `import scala.Nil` used here and unused once EmptyCollectionsUnified has
// written `List.empty` (E198); the second round, over the rewritten text, removes it, so a second run changes nothing.
package p
import scala.Nil
object Example { val empty: List[Int] = Nil }
