package gpa

// Given objects of a package and of an object published before their bodies run, read over the
// products: a method the body calls reads the given and gets the instance.
trait T { val self: T }
def helper: T = t
given t: T with
  val self: T = helper
object O:
  def helper: T = u
  given u: T with
    val self: T = helper
