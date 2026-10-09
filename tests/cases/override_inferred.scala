// An overriding member without a declared type takes the type of the member it overrides;
// protected constructor parameters implement protected members; a type constructor conforms
// to the bases it passes its parameter to; a member returning its own trait's type is read as
// this.type where a subclass narrows it.
trait Shape:
  def name: Any
  def sides: Option[Int]
  def scale: Long
  def describe(): String = s"$name with $sides sides, scale $scale"
class Square extends Shape:
  def name = "square"
  def sides = Some(4)
  def scale = 3
trait Coerce[A, B]:
  def unwrap: A => B
def coerce[B] = new Coerce[Option[B], B]:
  def unwrap = _.get
trait Break:
  protected val break: Int
  def show: String = s"break $break"
case class BreakImpl(protected val break: Int) extends Break
trait Base[+A]
trait Sub[A] extends Base[A]
trait Holder[+F[_]]:
  def held: Holder[F]
class Narrow extends Holder[Sub]:
  def held: Holder[Sub] = this
class Both extends Holder[Base]:
  def held: Holder[Sub] = Narrow()
trait Define[A]:
  def coll: Define[A]
  def s = coll
trait Iter[A] extends Define[A]:
  def coll: Iter[A] = this
trait Sup[A]:
  def coll: Iter[A]
class Foo[T] extends Iter[T] with Sup[T]
trait Api:
  def run(): Unit
  def size: Int
class Impl extends Api:
  def run() = println("running")
  def size = 5
@main def main(): Unit =
  val sq: Shape = Square()
  println(sq.describe())
  println(coerce[Int].unwrap(Some(7)))
  println(BreakImpl(2).show)
  println(Both().held.isInstanceOf[Narrow])
  println(Foo[Int]().s.isInstanceOf[Foo[?]])
  Impl().run()
  println(Impl().size + 1)
