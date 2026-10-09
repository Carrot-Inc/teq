package sharedlib.vdom

import sharedlib.tailwind.Tw

final case class Css(value: String)

sealed trait TagMod
final case class Attr(name: String, value: String) extends TagMod
final case class Text(value: String) extends TagMod
final case class Element(tag: String, mods: List[TagMod]) extends TagMod
final case class Mods(items: List[TagMod]) extends TagMod

trait Key[K]:
  def keyOf(k: K): String

object Key:
  given Key[Int] with
    def keyOf(k: Int): String = "k" + k.toString

object HtmlTags:
  def div(mods: TagMod*): Element = Element("div", mods.toList)
  def span(mods: TagMod*): Element = Element("span", mods.toList)
  def li(mods: TagMod*): Element = Element("li", mods.toList)

type ClassArg = String | (String, Boolean) | Tw | (Tw, Boolean)

object HtmlAttrs:
  object cls:
    def :=(classes: ClassArg*): TagMod =
      val names = classes.toList.map:
        case s: String => s
        case t: Tw => t.value
        case (s: String, on: Boolean) => if on then s else ""
        case (t: Tw, on: Boolean) => if on then t.value else ""
      Attr("class", names.filter(_.nonEmpty).mkString(" "))

  final class AttrKey(val name: String)

  extension (key: AttrKey)
    def :=(value: String): TagMod = Attr(key.name, value)

  final class StyleKey(val name: String)

  extension (key: StyleKey)
    def :=(declarations: Css*): TagMod = Attr(key.name, declarations.toList.map(_.value).mkString("; "))

  val title: AttrKey = AttrKey("title")
  val style: StyleKey = StyleKey("style")

object VdomSyntax:
  extension [A](items: List[A])
    def toKeyedNodes[K: Key](extractKey: A => K)
                           (f: A => Element): TagMod =
      Mods(items.map: a =>
        val el = f(a)
        el.copy(mods = Attr("key", summon[Key[K]].keyOf(extractKey(a))) :: el.mods)
      )

  def flatten(mods: List[TagMod]): List[TagMod] = mods.flatMap:
    case Mods(items) => flatten(items)
    case other => List(other)

  def render(mod: TagMod): String = mod match
    case Attr(name, value) => " " + name + "=\"" + value + "\""
    case Text(value) => value
    case Mods(items) => items.map(render).mkString("")
    case Element(tag, mods) =>
      val (attrs, children) = flatten(mods).partition:
        case Attr(_, _) => true
        case _ => false
      "<" + tag + attrs.map(render).mkString("") + ">" + children.map(render).mkString("") + "</" + tag + ">"
