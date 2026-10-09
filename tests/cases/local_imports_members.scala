@main def main(): Unit =
  val engine = Engine(Engine.State(1))
  println(engine.step(engine.current))
  println(engine.config)
  println(engine.bumped)
  println(Permissions.roleGrants(Permissions.Role.Admin))
  println(Permissions.roleGrants(Permissions.Role.Guest))
  println(Permissions.default)
  println(Buttons.classes(Buttons.Intent.Danger, Buttons.Size.Large))
  import Codecs.given
  println(Codecs.encode(Buttons.Size.Small))
  println(Widget.render)
  println(Lazy.first + Lazy.second)

object Engine:
  case class State(n: Int)
  final case class Config(label: String)
  def initial: State = State(0)
  def describe(s: State): String = s"state ${s.n}"

class Engine(start: Engine.State):
  import Engine.*
  var current: State = start
  def config: Config = Config("c" + start.n)
  def step(s: State): State = State(s.n + 1)
  def bumped = describe(step(initial))

object Permissions:
  enum Role:
    case Admin, Guest

  enum Permission:
    case Read, Write, Delete

  import Permission.*

  def roleGrants(role: Role): Set[Permission] = role match
    case Role.Admin => Set(
      Read,
      Write,
      Delete,
    )
    case Role.Guest => Set(Read)

  val default = Read

object Buttons:
  enum Intent:
    case Primary, Danger

  enum Size:
    case Small, Large

  import Intent.*
  import Size.*

  private val base = "btn"

  def classes(intent: Intent, size: Size): String =
    val color = intent match
      case Primary => "blue"
      case Danger => "red"
    val scale = size match
      case Small => "sm"
      case Large => "lg"
    s"$base $color $scale"

trait Encoder[A]:
  def encode(a: A): String

object Helpers:
  def sizeName(s: Buttons.Size): String = s.toString.toLowerCase
  def wrap(s: String): String = s"<$s>"

object Codecs:
  import Helpers.{sizeName, wrap}

  given Encoder[Buttons.Size] with
    def encode(s: Buttons.Size): String = wrap(sizeName(s))

  def encode[A](a: A)(using e: Encoder[A]): String = e.encode(a)

object Widget:
  object Parts:
    val header: String = "header"
    def footer(n: Int): String = s"footer $n"
  val render: String =
    import Parts.*
    List(header, footer(2)).mkString(" | ")

object Lazy:
  import Later.*
  val first = value
  lazy val second: Int = twice(value)

object Later:
  val value: Int = 21
  def twice(n: Int): Int = n * 2
