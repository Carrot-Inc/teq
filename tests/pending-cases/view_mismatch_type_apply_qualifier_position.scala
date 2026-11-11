// The view search's ambiguity is the qualifier's mismatch, reported at the qualifier's point (Typer.scala:4346-4353,
// `errorTree`): the qualifier `Option.empty` adapted to `Option.empty[Nothing]` (an inserted `TypeApply`) is reported
// from its start, as scalac 3.8.4 gives it. teq reports its own ambiguity at the selection; master (efddb38de3) and the
// head (61c1af7941) alike. Queued: push 6 (typeMismatch whole, at the qualifier, whose inserted application points at
// `empty`: inferred_type_apply_mismatch_point).
trait Ops:
  extension [T](r: Option[T]) def pick: String
object Main:
  given a: Ops with
    extension [T](r: Option[T]) def pick: String = "a"
  given b: Ops with
    extension [T](r: Option[T]) def pick: String = "b"
  def mk[T]: Option[T] = None
  def main(args: Array[String]): Unit =
    println(mk.pick)
    println(Option.empty.pick)
