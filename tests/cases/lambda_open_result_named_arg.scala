// The cons of an invariant class's values, one built in a `map` lambda, as a named argument to a
// generic method whose `T` is open: through an unannotated helper, whose inferred result is
// `List[Item[Option[Status]]]` (the lambda's `T` left open, fixed by the cons, then completed:
// `Namer.inferredResultType`), and written inline under the expected `List[Item[T]]`; `T` fixed by
// the other argument and by the declared result alike.
package lambdaopenresultnamedarg

final case class Item[T](title: String, value: T, disabled: Boolean = false)
enum Status(val label: String):
  case Open extends Status("Open")
  case Shipped extends Status("Shipped")
final case class Method(name: String)
final case class Picker[T](items: List[Item[T]])

def render[T](items: List[Item[T]], selected: T): String =
  items.map(i => (i.value == selected, i.value)).mkString(",")
def picker[T](items: List[Item[T]], title: String): Picker[T] = Picker(items)
def statusItems(method: Method) =
  Item[Option[Status]](method.name, None) :: Status.values.toList.map(s => Item(s.label, Some(s)))

@main def main(): Unit =
  val current: Option[Status] = Some(Status.Shipped)
  val method = Method("Any")
  println(render(items = statusItems(method), selected = current))
  println(render(selected = current, items = Item[Option[Status]](method.name, None) :: Status.values.toList.map(s => Item(s.label, Some(s)))))
  val p: Picker[Option[Status]] = picker(title = "status", items = statusItems(method))
  println(p.items.map(_.value))
  val q: Picker[Option[Status]] = picker(title = "inline", items = Item[Option[Status]]("Any", None) :: Status.values.toList.map(s => Item(s.label, Some(s))))
  println(q.items.map(_.value))
