// An export's forwarder of a stable val is a path as the val is (dotty's `StableRealizable`
// forwarder `final def onClick: Attrs.onClick.type`): its type members are selected through it,
// imported, selected on the exporting object, and through a top-level export.
trait Attr:
  type Value
  def name: String
object Attrs:
  val onClick: Attr { type Value = Int } = new Attr { type Value = Int; def name = "click" }
  val title: Attr { type Value = String } = new Attr { type Value = String; def name = "title" }
object Prelude:
  export Attrs.*
export Attrs.title

object UsesImport:
  import Prelude.*
  def handler(v: onClick.Value): String = s"${onClick.name}=$v"

def selected(v: Prelude.onClick.Value): Int = v + 1
def top(v: title.Value): String = s"${title.name}=$v"

@main def run(): Unit =
  println(UsesImport.handler(3))
  println(selected(4))
  println(top("t"))
