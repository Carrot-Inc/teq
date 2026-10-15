package sba

import scala.collection.immutable.SortedSet

// A member whose parameter names scala-library's `immutable.SortedSet`: a selection of it carries
// the signature of the declaration it selects (`TreePickler`'s `SELECTin`), the library's class by
// its own name, not `collection.SortedSet`, which the std binds to the same class.
object Sets:
  def first(s: SortedSet[Int]): Int = s.head
