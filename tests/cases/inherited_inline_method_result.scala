// An inline method of no written result type that overrides takes the overridden's result, and its body is typed
// against it: a lambda's parameter type comes from the inherited function type, and the call's type is the inherited
// CharSequence, not the body's String, which overloading sees.
trait Step {
  def next: Int => Int
  def name: CharSequence
}
class Inc extends Step {
  inline def next = x => x + 1
  inline def name = "inc"
}
def describe(s: String): String = "a String"
def describe(s: CharSequence): String = "a CharSequence"
@main def main(): Unit =
  val i = new Inc
  println(i.next(41))
  println(describe(i.name))
