// A def whose `=` is missing before an indented body: a declaration of a class that is not
// abstract, and the body's line a statement of the class.
class Loud(s: String):
  override def toString: String
    println(s)
