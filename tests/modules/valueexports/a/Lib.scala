package lib

class Info(val ply: Int, val color: String):
  def describe(prefix: String, n: Int = 1): String = s"$prefix $ply $color $n"
  def twice[A](a: A): List[A] = List(a, a)

trait HasInfo:
  val info: Info
  export info.{ ply, color, describe as desc }

case class Advice(info: Info, text: String) extends HasInfo

class Game(val chess: Info):
  export chess.*

trait Ext:
  def baseUrl: String
class Net(val baseUrl: String)
class Ctrl(net: Net) extends Ext:
  export net.baseUrl
