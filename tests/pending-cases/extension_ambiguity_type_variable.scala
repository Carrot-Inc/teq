// The qualifier's type is shown as `TypeMismatch` shows a found type, a constrained variable by its bound (`reported`,
// messages.scala:319-357): `Option[Nothing]` for `Option[T]`, in the view search's ambiguity (Typer.scala:4346-4353;
// scalac 3.8.4). teq reports its own ambiguity at the selection: master (efddb38de3) and the head (61c1af7941) "both a
// and b provide pick on Option[T]". Queued: push 6 (typeMismatch whole).
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
