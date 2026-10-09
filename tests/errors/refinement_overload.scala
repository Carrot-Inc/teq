// expect: 14:26: error: Refinements cannot introduce overloaded definitions
// expect: 15:30: error: Polymorphic refinement method run without matching type in parent type AnyRef is no longer allowed
// expect: 16:27: error: Refinements cannot introduce overloaded definitions
// expect: 16:47: error: Refinements cannot introduce overloaded definitions
// expect: 17:27: error: Refinements cannot introduce overloaded definitions
// A refinement makes a class of its members, where a member of the refinement's name is not
// overloaded and a generic one refines one of the parent's (dotty's `Typer.typedRefinedTypeTree`,
// scalac 3.8.4's E133 and E117): the generic `run` refines one of the parent's two, which
// overloads it; `one(x: Int)` refines no `one`; two `m`s overload each other.
trait Parent:
  def run[A](a: A): Any
  def run[A](a: A, n: Int): Any
  def one(x: String): Int
def keep(f: Parent { def run[A](a: A): A }) = f
def noParent(f: AnyRef { def run[A](a: A): A }) = f
def twice(f: AnyRef { def m(x: Int): Int; def m(x: String): Int }) = f
def other(f: Parent { def one(x: Int): Int }) = f
def fine(f: Parent { def one(x: String): Int }) = f
