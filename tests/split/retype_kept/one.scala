package kept.one

class Box[T <: String]

val x = 1

def f(a: Box[x.type]): Int = 0

/** An error that a retype reports again, the import's in a class's body, beside a val whose
  * body types clean and gives another type. */
object A {
  import kept.nowhere.X
  type T = Int
  def label: String = "a"
}
