package errs

class Box[T <: String]

/** A type argument of a signature against its bound: checked once, after the classes. */
class B:
  def g(x: Box[Int]): Int = 1
  def label: String = "b"
