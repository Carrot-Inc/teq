package library

// An explicit package object's name is `package`, even when its declarations
// use the same file holder as ordinary top-level definitions.
package object values:
  type Count = Int
  val initial: Count = 4
  var current: Count = initial
  def advance(by: Count): Count =
    current += by
    current
