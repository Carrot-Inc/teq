class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

def tail(): String =
  println("  tail")
  "!"

object Flags:
  final val on = true

object Inl:
  transparent inline def cat(inline x: Any): String = "" + x
  inline def plain(inline x: Any): String = "" + x

object Helpers:
  def plain(x: Any) = ("" + x) + tail()
  def constantIf(x: Any) = (if true then "" + x else "") + tail()
  def viaFinal(x: Any) = (if Flags.on then "" + x else "") + tail()
  def ascribed(x: Any) = (("" + x): String) + tail()
  def shown(x: Any) = ("" + x).toString + tail()
  def cast(x: Any) = ("" + x).asInstanceOf[String] + tail()
  def transparentHead(x: Any) = Inl.cat(x) + tail()
  def plainHead(x: Any) = Inl.plain(x) + tail()
  def block(x: Any) = ({ println("  stat"); "" + x }) + tail()
