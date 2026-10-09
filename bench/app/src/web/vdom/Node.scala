package meridian.web.vdom

import meridian.core.effect.Task

/** A virtual DOM: a tree of tags and text, with attributes, styles, event handlers, keys and
  * component nodes as the modifiers of a tag. Unions stand where the original library has
  * implicit conversions. */
final class VdomTag(val name: String, val mods: List[Mod]):
  def apply(more: Mod*): VdomTag = new VdomTag(name, mods ++ more.toList)
  def withKey(key: String): VdomTag = new VdomTag(name, KeyMod(key) :: mods)

final class Fragment(val children: List[Node])
final class EmptyNode
final class KeyedNodes(val items: List[(String, Node)])
final class ComponentNode(val name: String, val props: Any, val render: Any => Node, val key: Option[String] = None, val reuse: Option[(Any, Any) => Boolean] = None)

type Leaf = VdomTag | Fragment | ComponentNode | KeyedNodes | EmptyNode | String | Int | Long
type Node = Leaf | Option[Leaf]

final class AttrMod(val key: String, val value: String)
final class BoolAttrMod(val key: String, val on: Boolean)
final class StyleMod(val key: String, val value: String)
final class EventMod(val event: String, val handler: EventPayload => Task[Unit])
final class KeyMod(val key: String)
final class ModList(val mods: List[Mod])

type Mod = Node | AttrMod | BoolAttrMod | StyleMod | EventMod | KeyMod | ModList

/** What a fired event carries: the value of the target, if any. */
final class EventPayload(val value: String, val checked: Boolean):
  def preventDefault(): Unit = ()
  def stopPropagation(): Unit = ()

val EmptyVdom: EmptyNode = new EmptyNode()

object TagMod:
  def apply(mods: Mod*): ModList = new ModList(mods.toList)
  val empty: ModList = new ModList(Nil)
  def when(condition: Boolean)(mod: => Mod): Mod = if condition then mod else empty
  def unless(condition: Boolean)(mod: => Mod): Mod = when(!condition)(mod)

object ReactFragment:
  def apply(children: Node*): Fragment = new Fragment(children.toList)
  def withKey(key: String)(children: Node*): Fragment = new Fragment(children.toList)

extension (mod: Mod)
  def when(condition: Boolean): Mod = if condition then mod else TagMod.empty
  def unless(condition: Boolean): Mod = if condition then TagMod.empty else mod

extension [A](option: Option[A])
  def ifDefined(f: A => Mod): Mod = option match
    case Some(a) => f(a)
    case None => TagMod.empty
  def ifDefinedNode(f: A => Node): Node = option match
    case Some(a) => f(a)
    case None => EmptyVdom

extension [A](items: Seq[A])
  def toKeyedNodes(key: A => String)(f: A => Node): KeyedNodes = new KeyedNodes(items.toList.map(a => (key(a), f(a))))
  def toNodes(f: A => Node): ModList = new ModList(items.toList.map(f))

extension (mods: Seq[Mod])
  def toMod: ModList = new ModList(mods.toList)

def fragmentGate(condition: Boolean)(children: Node*): Node = if condition then new Fragment(children.toList) else EmptyVdom
