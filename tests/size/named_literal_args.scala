// Named arguments written out of parameter order: those with effects are bound to temporaries in
// the order written, a literal stays in place, also one a plain inline call expands to after the
// typing around it, as where every call expanded as typed
object Css:
  inline def wide: String = "w-full text-left"
  inline def gap: String = "mt-4"
  inline def pad: String = "p-6"
  inline def size: Int = 900

final case class Props(
    onClose: Option[String],
    title: String = "",
    isOpen: Boolean = false,
    className: String = "",
    titleClassName: String = "",
    maxWidth: Int = 0
)

var opened = 0
def open(): Boolean =
  opened += 1
  opened % 2 == 1
def label(s: String): String =
  println(s"label $s")
  s

@main def run(): Unit =
  println(Props(titleClassName = Css.wide, isOpen = open(), onClose = Some("close"), title = label("New")))
  println(Props(className = Css.gap, onClose = None, isOpen = open()))
  println(Props(maxWidth = Css.size, title = label("Wide"), onClose = None, className = Css.pad))
  println(Props(titleClassName = Css.wide, className = Css.gap, isOpen = open(), onClose = Some(label("x"))))
  println(Props(className = Css.pad, title = "plain", onClose = None))
  println(Props(maxWidth = Css.size, isOpen = open(), titleClassName = Css.wide, onClose = Some(label("y"))))
