// expect: 8:3: error: expected an expression, found '['
// A `[` that begins the line after an indented block begins a statement: it applies no type argument to the
// block. scalac: "expression expected but '[' found", at the end of the block's last line.
object O:
  def id[A](a: A): A = a
  def f: Int =
    id
  [Int](1)
