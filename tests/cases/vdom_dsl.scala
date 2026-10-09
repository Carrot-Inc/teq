// A miniature of a React vdom DSL: symbolic extension operators used infix, a custom string
// interpolator, multi-argument infix calls and parameter clauses continued on the next line.

final class Tw(val value: String):
  def isEmpty: Boolean = value.isEmpty
  def ++(other: Tw): Tw =
    if value.isEmpty then other
    else if other.value.isEmpty then this
    else Tw(value + " " + other.value)
  override def toString: String = "Tw(" + value + ")"

object Tw:
  val empty: Tw = Tw("")

extension (sc: StringContext)
  def tw(args: Any*): Tw =
    var out = ""
    var i = 0
    while i < sc.parts.length do
      out = out + sc.parts(i)
      if i < args.length then out = out + args(i).toString
      i += 1
    Tw(out)

final class Callback(val run: () => Unit):
  def *>(next: => Callback): Callback =
    val first = run
    new Callback(() =>
      first()
      next.run()
    )
  def >>(next: Callback): Callback = this *> next

object Callback:
  def apply(body: => Unit): Callback = new Callback(() => body)
  val empty: Callback = new Callback(() => ())

final case class Event(target: String)

sealed trait TagMod
final case class Attr(name: String, value: String) extends TagMod
final case class Listener(name: String, handler: Event => Callback) extends TagMod
final case class Text(value: String) extends TagMod
final case class Element(tag: String, mods: List[TagMod]) extends TagMod
final case class Mods(items: List[TagMod]) extends TagMod
case object NoMod extends TagMod

final class AttrKey(val name: String)
final class EventKey(val name: String)

extension (key: AttrKey)
  def :=(value: String): TagMod = Attr(key.name, value)
  def :=?(value: Option[String]): TagMod = value match
    case Some(v) => Attr(key.name, v)
    case None => NoMod

extension (key: EventKey)
  def -->(callback: => Callback): TagMod = Listener(key.name, _ => callback)
  def ==>(handler: Event => Callback): TagMod = Listener(key.name, handler)

extension [A](a: A)
  def ===(b: A): Boolean = a == b
  def =!=(b: A): Boolean = a != b

object cls:
  def :=(classes: (String | (String, Boolean) | Tw | (Tw, Boolean))*): TagMod =
    var out = ""
    classes.foreach: c =>
      val name = c match
        case s: String => s
        case t: Tw => t.value
        case (s: String, on: Boolean) => if on then s else ""
        case (t: Tw, on: Boolean) => if on then t.value else ""
      if name.nonEmpty then out = (if out.isEmpty then name else out + " " + name)
    Attr("class", out)
  def :=?(classes: Option[Tw]): TagMod = classes match
    case Some(t) => Attr("class", t.value)
    case None => NoMod

val href = AttrKey("href")
val title = AttrKey("title")
val onClick = EventKey("click")
val onChange = EventKey("change")

def div(mods: TagMod*): Element = Element("div", mods.toList)
def span(mods: TagMod*): Element = Element("span", mods.toList)
def button(mods: TagMod*): Element = Element("button", mods.toList)

def label(text: String, style: Tw = tw"text-sm"): Element =
  span(cls := style, Text(text))

def badge(count: Int, style: Tw = tw"badge badge-${"default"}", prefix: String = s"#"): Element =
  span(cls := (style, tw"w-${count}"), Text(prefix + count.toString))

def pagePanel(className: Tw = Tw.empty)
               (children: TagMod*): Element =
  Element("section", (cls := (className, "p-4")) :: children.toList)

trait Show[A]:
  def show(a: A): String

given Show[Int] with
  def show(a: Int): String = "k" + a.toString

extension [A](items: List[A])
  def toKeyedNodes[K: Show](extractKey: A => K)
                          (f: A => TagMod): List[TagMod] =
    items.map(a => Mods(List(Attr("key", summon[Show[K]].show(extractKey(a))), f(a))))

def render(mod: TagMod): String = mod match
  case Attr(name, value) => " " + name + "=\"" + value + "\""
  case Listener(name, _) => " on:" + name
  case Text(value) => value
  case Mods(items) => items.map(render).mkString("")
  case NoMod => ""
  case Element(tag, mods) =>
    val flat = mods.flatMap:
      case Mods(items) => items
      case other => List(other)
    val attrs = flat.filter:
      case Attr(_, _) | Listener(_, _) => true
      case _ => false
    val children = flat.filter:
      case Attr(_, _) | Listener(_, _) => false
      case _ => true
    "<" + tag + attrs.map(render).mkString("") + ">" + children.map(render).mkString("") + "</" + tag + ">"

def fire(mod: TagMod, name: String, event: Event): Unit = mod match
  case Listener(n, handler) if n == name => handler(event).run()
  case Element(_, mods) => mods.foreach(m => fire(m, name, event))
  case Mods(items) => items.foreach(m => fire(m, name, event))
  case _ => ()

def log(message: String): Callback = Callback(println("log: " + message))

def handler(e: Event): Callback = log("changed " + e.target)

@main def main(): Unit =
  val active = true
  val wide = false
  val n = 4

  println(tw"text-sm font-bold")
  println(tw"w-${n} px-$n")
  println(tw"grid-cols-${n + 1}" ++ Tw.empty ++ tw"gap-2")

  println(render(div(cls := "flex")))
  println(render(div(cls := ("flex items-center", "font-bold" -> active, if wide then tw"w-full" else Tw.empty))))
  println(render(div(cls := ("a" -> false, tw"b" -> true, tw"c"))))
  println(render(div(cls := (("only-pair", active)))))
  println(render(div(cls :=? Some(tw"maybe"), cls :=? None)))
  println(render(div(href := "/home", title :=? Some("Home"), title :=? None)))

  var clicks = 0
  val view = div(
    cls := (tw"p-2", "selected" -> (clicks === 0)),
    button(
      onClick --> (log("clicked") *> Callback(clicks += 1) *> log("done")),
      Text("Save")
    ),
    span(
      onChange ==> (e => handler(e)),
      onClick ==> handler,
      onClick --> log("span " + clicks.toString)
    )
  )
  println(render(view))
  println("before: " + clicks.toString)
  fire(view, "click", Event("button"))
  fire(view, "click", Event("button"))
  fire(view, "change", Event("input"))
  println("after: " + clicks.toString)

  val chained = log("one") >> log("two") *> log("three")
  chained.run()

  println(render(label("plain")))
  println(render(label("styled", tw"text-lg")))
  println(render(badge(3)))
  println(render(badge(5, prefix = "no. ")))
  println(render(pagePanel()(Text("empty class"))))
  println(render(pagePanel(tw"bg-white")(label("a"), label("b"))))
  println(render(div(List(1, 2).toKeyedNodes(_ * 10)(i => Text(i.toString))*)))

  println(1 === 1)
  println("a" =!= "b")
  println((1 + 1 === 2) && (n =!= 5))
