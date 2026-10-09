// jars: sourcecode
// A facade of @jsImport bindings and a vdom-style DSL, re-exported through a prelude object.
package facade

@jsImport("node:path", "join")
def join(parts: String*): String

@jsImport("node:util", "format")
def format(template: String, args: Any*): String

@jsImport("node:path", "sep")
val separator: String

final case class Tw(value: String)

object Syntax:
  extension (sc: StringContext)
    def tw(args: Any*): Tw = Tw(sc.s(args*))

  object cls:
    def :=(classes: (String | (String, Boolean) | Tw)*): Any =
      val names = classes.toList.map:
        case s: String => s
        case t: Tw => t.value
        case (s: String, on: Boolean) => if on then s else ""
      js.obj("className" -> names.filter(n => n.nonEmpty).mkString(" "))

  def fc(render: => Any)(using name: sourcecode.FullName): Any =
    js.obj("displayName" -> name.value, "render" -> (() => render))
