// A sequence pattern's rest over a value whose `toSeq` and `drop` nothing has called before: a case class's
// repeated field, with and without elements before the rest, and a rest alone over an indexed sequence.
case class Foo(name: String, children: Int*)

@main def run(): Unit =
  val f = new Foo("hello", 3, 5)
  f match
    case Foo(name, ns*) => println(name + " " + ns.toList)
  f match
    case Foo(name, x, y, ns*) => println(s"$x $y ${ns.isEmpty}")
  Seq(1, 2, 3, 4).toIndexedSeq match
    case Seq(xs*) => println(xs.length)
