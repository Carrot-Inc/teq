// A same-named Java method of other parameters is no method a nullary one overrides: `charAt()` finds no alternative
// (`charAt(i: Int)` needs its argument, the nullary `charAt` takes no `()`), as dotty's `matchNullaryLoosely` asks of
// the overridden symbols. scalac reports the missing argument (E171).
// expect: 10:27: error: None of the overloaded alternatives of method charAt in class C with types
class C extends CharSequence:
  def length: Int = 1
  def charAt(i: Int): Char = 'a'
  def subSequence(a: Int, b: Int): CharSequence = "a"
  def charAt: Char = 'z'
@main def run() = println(new C().charAt())
