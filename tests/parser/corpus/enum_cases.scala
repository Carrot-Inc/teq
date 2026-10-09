enum Cmd:
  case Quit
  case Say[T](what: T, times: Int = 1)
  case Move(dx: Int, dy: Int)(val label: String)

  def run: String = this match
    case Quit => "bye"
    case Say(what, times) => (what.toString + " ") * times
    case m: Move => s"move ${m.dx},${m.dy} ${m.label}"

enum Tree[+A]:
  case Leaf
  case Node(left: Tree[A], value: A, right: Tree[A])

  def size: Int = this match
    case Leaf => 0
    case Node(l, _, r) => l.size + 1 + r.size

  def insert[B >: A](x: B)(using ord: Ordering[B]): Tree[B] = this match
    case Leaf => Node(Leaf, x, Leaf)
    case Node(l, v, r) =>
      if ord.lt(x, v) then Node(l.insert(x), v, r)
      else Node(l, v, r.insert(x))

  def toList: List[A] = this match
    case Leaf => Nil
    case Node(l, v, r) => l.toList ++ (v :: r.toList)

enum Sink[-T]:
  case Null
  case Print(prefix: String)
  def accept(t: T): String = this match
    case Null => ""
    case Print(p) => p + t

@main def main(): Unit =
  println(Cmd.Quit.run)
  println(Cmd.Say("hi", 2).run)
  println(Cmd.Say(3).run)
  println(Cmd.Move(1, 2)("left").run)
  println(Cmd.Move(1, 2)("a") == Cmd.Move(1, 2)("b"))
  val cmds = for i <- List(1, 2) yield Cmd.Say(i, i)
  println(cmds.map(_.run))
  val t = List(5, 2, 8, 1).foldLeft(Tree.Leaf: Tree[Int])((acc, x) => acc.insert(x))
  println(t.toList)
  println(t.size)
  println(t match
    case Tree.Node(_, v, _) => v
    case Tree.Leaf => -1)
  val sink: Sink[Int] = Sink.Print("> ")
  println(sink.accept(4))
  val anySink: Sink[Any] = Sink.Null
  val narrowed: Sink[String] = anySink
  println(narrowed.accept("x").isEmpty)
  println(Cmd.Say("x").ordinal + Cmd.Quit.ordinal)
