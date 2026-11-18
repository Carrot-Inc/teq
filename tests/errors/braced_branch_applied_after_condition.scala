// expect: 11:5: error: expected end of statement, found '('
// A block in braces is no function an argument list applies: after `if (c)` a branch written as a block in
// braces ends where its braces close, and an argument list on the next line, further in, is refused. scalac:
// "end of statement expected but '(' found" there. (Listed: teq applies the block to the arguments; it applies one
// on the branch's own line too, which scalac refuses as well.)
class V:
  def apply(x: Int): Int = x + 10

def f(c: Boolean): Int =
  if (c) { new V }
    (2)
  else 0

@main def main(): Unit = println(f(true))
