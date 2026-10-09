// An inline method that overrides one that is not keeps a dispatch method for the calls through
// the overridden member, which scalac makes by inlining the body into it with its own
// parameters: its `inline match` reduces on their declared types (`x: Any` matches no
// `case _: Int`), where a call of the inline method reduces on its argument's.
trait Base:
  def pick(x: Any): String
  def size(xs: Seq[Int]): String

object Ret extends Base:
  override inline def pick(x: Any): String = inline x match
    case _: Int => "int"
    case _ => "other"
  override inline def size(xs: Seq[Int]): String = inline xs match
    case _: List[Int] => "list"
    case _: Seq[Int] => "seq"

@main def run(): Unit =
  val b: Base = Ret
  println(b.pick(1))
  println(Ret.pick(1))
  println(b.size(List(1)))
  println(Ret.size(List(1)))
