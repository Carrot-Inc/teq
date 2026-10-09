package meridian.web.vdom

import meridian.core.effect.{Eff, Task}

final class Attr(val key: String):
  def :=(value: String | Int | Long | Boolean): Mod = value match
    case b: Boolean => new BoolAttrMod(key, b)
    case other => new AttrMod(key, other.toString)
  def :=?(value: Option[String | Int | Long | Boolean]): Mod = value match
    case Some(v) => this := v
    case None => TagMod.empty

final class Style(val key: String):
  def :=(value: String | Int | Long): StyleMod = new StyleMod(key, value.toString)
  def :=?(value: Option[String]): Mod = value match
    case Some(v) => new StyleMod(key, v)
    case None => TagMod.empty

final class Event(val name: String):
  /** A handler that ignores the event. */
  def -->(callback: => Task[Unit]): EventMod = new EventMod(name, _ => callback)
  /** A handler that reads the event. */
  def ==>(handler: EventPayload => Task[Unit]): EventMod = new EventMod(name, handler)
  /** A handler that reads the target's value, as text inputs do. */
  def ===>(handler: String => Task[Unit]): EventMod = new EventMod(name, e => handler(e.value))
  def -->?(callback: Option[Task[Unit]]): Mod = callback match
    case Some(cb) => new EventMod(name, _ => cb)
    case None => TagMod.empty

object Attrs:
  val vdomCls: Attr = new Attr("class")
  val id: Attr = new Attr("id")
  val value: Attr = new Attr("value")
  val disabled: Attr = new Attr("disabled")
  val checked: Attr = new Attr("checked")
  val placeholder: Attr = new Attr("placeholder")
  val key: Attr = new Attr("key")
  val tpe: Attr = new Attr("type")
  val name: Attr = new Attr("name")
  val src: Attr = new Attr("src")
  val alt: Attr = new Attr("alt")
  val href: Attr = new Attr("href")
  val title: Attr = new Attr("title")
  val min: Attr = new Attr("min")
  val max: Attr = new Attr("max")
  val step: Attr = new Attr("step")
  val required: Attr = new Attr("required")
  val readOnly: Attr = new Attr("readonly")
  val autoFocus: Attr = new Attr("autofocus")
  val tabIndex: Attr = new Attr("tabindex")
  val htmlFor: Attr = new Attr("for")
  val hidden: Attr = new Attr("hidden")
  val colSpan: Attr = new Attr("colspan")
  val rows: Attr = new Attr("rows")
  val role: Attr = new Attr("role")
  val target: Attr = new Attr("target")
  val draggable: Attr = new Attr("draggable")
  val selected: Attr = new Attr("selected")
  val cy: Attr = new Attr("data-cy")
  val dataTestId: Attr = new Attr("data-testid")
  def data(name: String): Attr = new Attr(s"data-$name")
  object aria:
    val label: Attr = new Attr("aria-label")
    val pressed: Attr = new Attr("aria-pressed")
    val busy: Attr = new Attr("aria-busy")
    val hidden: Attr = new Attr("aria-hidden")

object Styles:
  val height: Style = new Style("height")
  val width: Style = new Style("width")
  val minWidth: Style = new Style("min-width")
  val maxWidth: Style = new Style("max-width")
  val minHeight: Style = new Style("min-height")
  val maxHeight: Style = new Style("max-height")
  val backgroundColor: Style = new Style("background-color")
  val color: Style = new Style("color")
  val fontSize: Style = new Style("font-size")
  val padding: Style = new Style("padding")
  val margin: Style = new Style("margin")
  val marginTop: Style = new Style("margin-top")
  val marginLeft: Style = new Style("margin-left")
  val top: Style = new Style("top")
  val left: Style = new Style("left")
  val opacity: Style = new Style("opacity")
  val display: Style = new Style("display")
  val position: Style = new Style("position")
  val zIndex: Style = new Style("z-index")
  val transform: Style = new Style("transform")
  val gridTemplateColumns: Style = new Style("grid-template-columns")

object Events:
  val onClick: Event = new Event("click")
  val onChange: Event = new Event("change")
  val onInput: Event = new Event("input")
  val onSubmit: Event = new Event("submit")
  val onFocus: Event = new Event("focus")
  val onBlur: Event = new Event("blur")
  val onKeyDown: Event = new Event("keydown")
  val onMouseDown: Event = new Event("mousedown")
  val onMouseEnter: Event = new Event("mouseenter")
  val onMouseLeave: Event = new Event("mouseleave")
  val onScroll: Event = new Event("scroll")
  val onDragStart: Event = new Event("dragstart")
  val onDrop: Event = new Event("drop")

extension (n: Int) def px: String = if n == 0 then "0" else s"${n}px"
extension (n: Long) def pxl: String = if n == 0L then "0" else s"${n}px"

extension (payload: EventPayload)
  def preventDefaultIO: Task[Unit] = Eff.unit
  def stopPropagationIO: Task[Unit] = Eff.unit
