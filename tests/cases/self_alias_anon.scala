// A member declared in an anonymous class that extends the class it is nested in: `Validate.this`
// in its own declaration names the enclosing instance (`val self = this` outside), not the
// anonymous class, as in refined's `Validate.contramap`.
trait Validate[T]:
  type R
  type Res = Option[R]
  def validate(t: T): Res
  def contramap[U](f: U => T): Validate[U] { type R = Validate.this.R } =
    val self: Validate[T] { type R = Validate.this.R } = this
    new Validate[U]:
      type R = self.R
      def validate(u: U): Res = self.validate(f(u))

object Positive extends Validate[Int]:
  type R = String
  def validate(t: Int): Res = if t > 0 then Some("positive") else None

object Main:
  def main(args: Array[String]): Unit =
    val w = Positive.contramap[String](_.length)
    println(w.validate("abc").toString + " " + w.validate(""))
    val r: Option[String] = w.validate("x")
    println(r)
