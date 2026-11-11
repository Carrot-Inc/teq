// A repeated argument is adapted to its element type twice (Applications.matchArgs: `typedArg`, then `addArg`). Where
// the element type is a type variable still open at the first adaptation, the second one, made once the variable is
// solved, is where a generic result gets the cast to its primitive type.
enum Flags(val all: List[Boolean]):
  def this(one: Boolean) = this(List(one))
  case One extends Flags(true)
  case Two extends Flags(List(One.all.head, !One.all.head))

def ends(xs: List[Boolean]): List[Boolean] = List(xs.head, xs.last)
def anyEnds(xs: List[Boolean]): Seq[Any] = Seq(xs.head, xs.last)
def units(xs: List[Unit]): List[Unit] = List(xs.head, xs.last)
def chars(xs: List[Char]): List[Char] = List(xs.head, xs.last)

@main def run(): Unit =
  println(Flags.Two.all)
  println(ends(List(true, false, false)))
  println(anyEnds(List(false, true)))
  println(units(List((), ())).size)
  println(chars(List('a', 'b', 'c')))
  println(ends(List(true)).map(!_))
