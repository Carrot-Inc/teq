package mgg

trait TypeName[A]
object TypeName:
  given TypeName[String] = new TypeName[String] {}
  given TypeName[Uri] = new TypeName[Uri] {}

trait Show[A]
object Show:
  given Show[String] = new Show[String] {}

final class Get[A](val read: String => A):
  def temap[B](f: A => Either[String, B])(using sh: Show[A], tb: TypeName[B]): Get[B] =
    new Get(s => f(read(s)).fold(e => throw new Exception(e), b => b))
object Get:
  def apply[A](using ev: Get[A]): ev.type = ev
  given metaProjection[A](using m: Meta[A]): Get[A] = m.get

final class Put[A](val write: A => String)

final class Meta[A](val get: Get[A], val put: Put[A]):
  def tiemap[B](f: A => Either[String, B])(g: B => A)(using tb: TypeName[B], sh: Show[A]): Meta[B] =
    new Meta(get.temap(f), new Put(b => put.write(g(b))))
object Meta:
  def apply[A](using ev: Meta[A]): ev.type = ev
  given StringMeta: Meta[String] = new Meta(new Get(s => s), new Put(s => s))

case class Uri(s: String)
object Uri:
  def parse(s: String): Either[String, Uri] = Right(Uri(s))

// The summoner applied with its given passed as a plain list, as scalac pickles it: the given's
// body, which the downstream module's macro reaches through the interpreter, types it as the
// summoner, not the class's constructor (the application's m6 over m5's products).
object DatabaseUtils:
  given Meta[Uri] = Meta[String].tiemap(Uri.parse)(_.s)
  given Get[Uri] = Get[String].temap(Uri.parse)
