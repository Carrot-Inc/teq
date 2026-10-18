// An extension method named as Object's wait or notify, whose arguments no overload of Object's takes: the member's
// application fails, and the qualifier's adaptation finds the extension (dotty's Applications and
// Typer.tryInsertImplicitOnQualifier), as scalac does, on every target.
class Gate
extension (g: Gate)
  def wait(label: String): Unit = println(label)
  def notify(label: String): Unit = println(label)
object Main:
  def main(args: Array[String]): Unit =
    new Gate().wait("extension wait")
    new Gate().notify("extension notify")
