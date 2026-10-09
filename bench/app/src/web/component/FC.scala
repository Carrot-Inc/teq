package meridian.web.component

import meridian.web.vdom.{ComponentNode, Node}

/** A function component: its display name comes from the val it is assigned to. */
final class FC[P](val name: String, val render: P => Node, val reuse: Option[(P, P) => Boolean] = None):
  private val reuseAny: Option[(Any, Any) => Boolean] = reuse.map(test => (a, b) => test(a.asInstanceOf[P], b.asInstanceOf[P]))
  def apply(props: P): Node = new ComponentNode(name, props, p => render(p.asInstanceOf[P]), None, reuseAny)
  def withKey(key: String)(props: P): Node = new ComponentNode(name, props, p => render(p.asInstanceOf[P]), Some(key), reuseAny)
  def withDisplayName(displayName: String): FC[P] = new FC(displayName, render, reuse)

object FC:
  def apply[P](render: P => Node)(using name: sourcecode.Name): FC[P] = new FC(displayName(name.value), render)
  def withChildren[P](render: (P, List[Node]) => Node)(using name: sourcecode.Name): FCOverChildren[P] =
    new FCOverChildren(displayName(name.value), render)
  def memo[P](component: FC[P])(using reuse: Reusability[P]): FC[P] =
    new FC(component.name, component.render, Some(reuse.test))
  def displayName(value: String): String = if value.endsWith("Component") then value.dropRight(9) else value

final class FCOverChildren[P](val name: String, val render: (P, List[Node]) => Node):
  def apply(props: P)(children: Node*): Node =
    new ComponentNode(name, props, p => render(p.asInstanceOf[P], children.toList))

/** A component without props. */
final class FCN(val name: String, val render: () => Node):
  def apply(): Node = new ComponentNode(name, (), _ => render())

object FCN:
  def apply(body: => Node)(using name: sourcecode.Name): FCN = new FCN(FC.displayName(name.value), () => body)
  def withChildren(render: List[Node] => Node)(using name: sourcecode.Name): FCOverChildren[Unit] =
    new FCOverChildren(FC.displayName(name.value), (_, children) => render(children))

/** An inline component whose name is the enclosing definition and line. */
def fc(body: => Node)(using fullName: sourcecode.FullName, line: sourcecode.Line): Node =
  val name = fullName.value.split('.').last + ":" + line.value
  new ComponentNode(name, (), _ => body)

def memoBy[P, R](component: FC[P], by: P => R)(using reuse: Reusability[R]): FC[P] =
  FC.memo(component)(using Reusability.by(by))
