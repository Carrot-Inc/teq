package infernestedexpected

// A type variable of an enclosing call reaches a lambda nested in another call: `Column[T]`
// inside `List(...)` inside `GridProps[T](...)`, where `data` settles `T`.

final case class Column[T](key: String, accessor: T => String, className: String = "")
final case class GridProps[T](data: List[T], columns: List[Column[T]], rowKey: T => String)
final case class Line(description: String, rate: Int)
final case class Handler[A](name: String, run: A => Int)
final case class Screen[A](items: List[A], handlers: Map[String, Handler[A]], fallback: Option[Handler[A]])

def render[T](props: GridProps[T]): String =
  props.data.map(d => props.rowKey(d) + ": " + props.columns.map(c => c.accessor(d)).mkString("|")).mkString("\n")

@main def main(): Unit =
  val lines = List(Line("a", 1), Line("b", 2))
  println(render(GridProps(
    data = lines,
    columns = List(
      Column("description", _.description, className = "x"),
      Column("rate", _.rate.toString),
    ),
    rowKey = _.description,
  )))
  println(render(GridProps(lines, List(Column("d", _.description)), _.description)))
  println(render(GridProps(lines, List(Column("r", _.rate.toString), Column("d", l => l.description * 2)), rowKey = _.rate.toString)))
  val screen = Screen(
    List("one", "three"),
    Map("len" -> Handler("len", _.length), "first" -> Handler("first", _.head.toInt)),
    Some(Handler("zero", _ => 0)),
  )
  println(screen.items.map(i => screen.handlers.values.map(_.run(i)).sum))
  println(screen.fallback.map(_.run("x")))
