// lila: setup/SetupForm.scala:71 (`mapping(..., "color" -> lila.common.Form.empty)(HookConfig.from)(_.>>)`)
// The T of `empty[T]: Field[Option[T]]` occurs invariantly, so it is not instantiated after the
// first argument list; the later lists (apply, unapply) fix it to String.
class Field[T](val name: String)
object Forms:
  def empty[T]: Field[Option[T]] = Field("empty")
  def number: Field[Int] = Field("number")
  def mapping[R, A1, A2](a1: (String, Field[A1]), a2: (String, Field[A2]))(
      apply: (A1, A2) => R
  )(unapply: R => Option[(A1, A2)]): (String, R => Option[(A1, A2)]) = (s"${a1._1},${a2._1}", unapply)

case class Hook(n: Int, color: Option[String]):
  def >> = Some((n, color))

@main def run() =
  import Forms.*
  val (names, unapply) = mapping("n" -> number, "color" -> empty)(Hook.apply)(_.>>)
  println(names)
  println(unapply(Hook(1, Some("white"))))
