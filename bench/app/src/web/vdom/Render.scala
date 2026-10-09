package meridian.web.vdom

import meridian.core.effect.{Eff, Exit, Log, Runtime, Task}

/** Renders a tree to text and keeps what a page keeps between renders: the hook slots of every
  * mounted component, the handlers of every element, and the effects to run after a render.
  * A self-test mounts a tree, fires events by element path and reads the text back. */
object Renderer:
  final class Context(val path: String):
    var slot: Int = 0
  private var contexts: List[Context] = Nil
  private var slots: Map[String, Vector[Any]] = Map.empty
  private var handlers: Map[String, EventPayload => Task[Unit]] = Map.empty
  private var effects: List[Task[Unit]] = Nil
  private var dirty: Boolean = false
  private var rendered: Int = 0

  def current: Context = contexts.headOption.getOrElse(throw new IllegalStateException("a hook outside a component"))
  def path: String = contexts.headOption.map(_.path).getOrElse("")

  def slotValue(context: Context, init: => Any): (Int, Any) =
    val i = context.slot
    context.slot += 1
    val values = slots.getOrElse(context.path, Vector.empty)
    if i < values.length then (i, values(i))
    else
      val v = init
      slots = slots.updated(context.path, values :+ v)
      (i, v)

  def setSlot(path: String, index: Int, value: Any, quiet: Boolean = false): Unit =
    slots.get(path).foreach(values => slots = slots.updated(path, values.updated(index, value)))
    if !quiet then dirty = true

  def readSlot(path: String, index: Int): Any = slots(path)(index)
  def invalidate(): Unit = dirty = true
  def schedule(effect: Task[Unit]): Unit = effects = effect :: effects
  def renders: Int = rendered

  def reset(): Unit =
    contexts = Nil
    slots = Map.empty
    handlers = Map.empty
    effects = Nil
    dirty = false
    rendered = 0

  def render(node: Node): String =
    handlers = Map.empty
    val out = new StringBuilder
    rendered += 1
    write(node, out, "", 0)
    out.toString

  /** Renders, runs the effects, and renders again while a state changed, a bounded number of times. */
  def mount(node: Node, rounds: Int = 6): String =
    var text = render(node)
    var round = 0
    while round < rounds && (effects.nonEmpty || dirty) do
      dirty = false
      runEffects()
      if dirty then text = render(node)
      round += 1
    text

  def fire(elementPath: String, event: String, payload: EventPayload = new EventPayload("", false)): Boolean =
    handlers.get(s"$elementPath@$event") match
      case Some(handler) =>
        Runtime.unsafeRun(handler(payload)) match
          case Exit.Failure(cause) => Log.error(s"handler $elementPath@$event: ${cause.prettyPrint}")
          case _ => ()
        true
      case None => false

  def handlerPaths: List[String] = handlers.keys.toList.sorted

  private def runEffects(): Unit =
    val pending = effects.reverse
    effects = Nil
    for effect <- pending do
      Runtime.unsafeRun(effect) match
        case Exit.Failure(cause) => Log.error(s"effect: ${cause.prettyPrint}")
        case _ => ()

  private def write(node: Node, out: StringBuilder, parent: String, index: Int): Int = node match
    case t: VdomTag =>
      writeTag(t, out, s"$parent/${t.name}[$index]")
      index + 1
    case f: Fragment =>
      var i = index
      for child <- f.children do i = write(child, out, parent, i)
      i
    case c: ComponentNode =>
      val context = new Context(s"$parent/${c.name}[$index]")
      contexts = context :: contexts
      val body =
        try c.render(c.props)
        finally contexts = contexts.tail
      write(body, out, context.path, 0)
    case k: KeyedNodes =>
      var i = index
      for (key, child) <- k.items do i = write(child, out, s"$parent/$key", i)
      i
    case _: EmptyNode => index
    case s: String =>
      escape(s, out)
      index
    case i: Int =>
      out.append(i)
      index
    case l: Long =>
      out.append(l)
      index
    case o: Option[?] =>
      o match
        case Some(leaf) => write(leaf.asInstanceOf[Node], out, parent, index)
        case None => index

  private def writeTag(tag: VdomTag, out: StringBuilder, path: String): Unit =
    var attrs: List[(String, String)] = Nil
    var classes: List[String] = Nil
    var styles: List[(String, String)] = Nil
    var children: List[Node] = Nil
    def collect(mod: Mod): Unit = mod match
      case a: AttrMod => if a.key == "class" then classes = classes :+ a.value else attrs = attrs.filterNot(_._1 == a.key) :+ (a.key, a.value)
      case b: BoolAttrMod => attrs = if b.on then attrs.filterNot(_._1 == b.key) :+ (b.key, "") else attrs.filterNot(_._1 == b.key)
      case s: StyleMod => styles = styles.filterNot(_._1 == s.key) :+ (s.key, s.value)
      case e: EventMod =>
        val key = s"$path@${e.event}"
        val previous = handlers.get(key)
        handlers = handlers.updated(key, previous match
          case Some(first) => (p: EventPayload) => first(p).flatMap(_ => e.handler(p))
          case None => e.handler)
      case k: KeyMod => attrs = attrs.filterNot(_._1 == "key") :+ ("key", k.key)
      case l: ModList => l.mods.foreach(collect)
      case n: (VdomTag | Fragment | ComponentNode | KeyedNodes | EmptyNode | String | Int | Long | Option[?]) => children = children :+ n.asInstanceOf[Node]
    tag.mods.foreach(collect)
    out.append('<').append(tag.name)
    if classes.nonEmpty then out.append(" class=\"").append(classes.filter(_.nonEmpty).mkString(" ")).append('"')
    for (k, v) <- attrs do
      out.append(' ').append(k)
      if v.nonEmpty then
        out.append("=\"")
        escape(v, out)
        out.append('"')
    if styles.nonEmpty then out.append(" style=\"").append(styles.map((k, v) => s"$k:$v").mkString(";")).append('"')
    out.append('>')
    var i = 0
    for child <- children do i = write(child, out, path, i)
    out.append("</").append(tag.name).append('>')

  private def escape(s: String, out: StringBuilder): Unit =
    var i = 0
    while i < s.length do
      s.charAt(i) match
        case '<' => out.append("&lt;")
        case '>' => out.append("&gt;")
        case '&' => out.append("&amp;")
        case '"' => out.append("&quot;")
        case c => out.append(c)
      i += 1
