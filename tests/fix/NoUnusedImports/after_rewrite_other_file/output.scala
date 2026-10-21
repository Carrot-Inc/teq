// Round two over a program where another file stays as it was: there the semantic rules alone run again, the others'
// results being round one's, so `Other.scala` keeps its `var` reported and its anchor's suppression, no anchor unused.
package p
object Example { val empty: List[Int] = List.empty }
